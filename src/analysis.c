/*
 * analysis.c - Data analysis functions for batlab
 * 
 * Functions for loading, parsing, and analyzing telemetry data files
 * to generate run summaries and statistics.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <math.h>
#include <dirent.h>
#include <sys/stat.h>
#include <ctype.h>
#include <errno.h>

#include "telemetry.h"

// Forward declarations
static int parse_jsonl_file(const char *jsonl_path, telemetry_sample_t **samples, int *count);
static int parse_json_line(const char *line, telemetry_sample_t *sample);
static double parse_json_double(const char *json, const char *key);
static const char *parse_json_string(const char *json, const char *key, char *buffer, size_t buffer_size);
static int compare_doubles(const void *a, const void *b);
static void parse_run_id_fallback(const char *run_id, char *config, char *workload, char *os);
static double calculate_duration(const telemetry_sample_t *samples, int count);

int load_run_summaries(const char *data_dir, int min_samples, run_summary_t **summaries, int *count) {
    if (!data_dir || !summaries || !count) {
        return -1;
    }
    
    *summaries = NULL;
    *count = 0;
    
    DIR *dir = opendir(data_dir);
    if (!dir) {
        return -1;
    }
    
    // First pass: count .jsonl files
    struct dirent *entry;
    int jsonl_count = 0;
    
    while ((entry = readdir(dir)) != NULL) {
        if (strstr(entry->d_name, ".jsonl")) {
            jsonl_count++;
        }
    }
    
    if (jsonl_count == 0) {
        closedir(dir);
        return 0;
    }
    
    // Allocate memory for summaries
    *summaries = malloc(jsonl_count * sizeof(run_summary_t));
    if (!*summaries) {
        closedir(dir);
        return -1;
    }
    
    // Second pass: analyze each file
    rewinddir(dir);
    int valid_runs = 0;
    
    while ((entry = readdir(dir)) != NULL) {
        if (strstr(entry->d_name, ".jsonl")) {
            char jsonl_path[MAX_PATH_LENGTH];
            snprintf(jsonl_path, sizeof(jsonl_path), "%s/%s", data_dir, entry->d_name);
            
            run_summary_t summary;
            if (analyze_run(jsonl_path, min_samples, &summary) == 0) {
                (*summaries)[valid_runs++] = summary;
            }
        }
    }
    
    closedir(dir);
    *count = valid_runs;
    
    // Shrink array to actual size
    if (valid_runs > 0 && valid_runs < jsonl_count) {
        *summaries = realloc(*summaries, valid_runs * sizeof(run_summary_t));
    }
    
    return 0;
}

int analyze_run(const char *jsonl_path, int min_samples, run_summary_t *summary) {
    if (!jsonl_path || !summary) {
        return -1;
    }
    
    // Initialize summary
    memset(summary, 0, sizeof(run_summary_t));
    
    // Extract run ID from filename
    const char *filename = strrchr(jsonl_path, '/');
    if (filename) {
        filename++; // Skip the '/'
    } else {
        filename = jsonl_path;
    }
    
    strncpy(summary->run_id, filename, sizeof(summary->run_id) - 1);
    summary->run_id[sizeof(summary->run_id) - 1] = '\0';
    
    // Remove .jsonl extension
    char *ext = strstr(summary->run_id, ".jsonl");
    if (ext) {
        *ext = '\0';
    }
    
    // Load telemetry samples
    telemetry_sample_t *samples = NULL;
    int sample_count = 0;
    
    if (parse_jsonl_file(jsonl_path, &samples, &sample_count) != 0) {
        return -1;
    }
    
    if (sample_count < min_samples) {
        free(samples);
        return -1;
    }
    
    summary->samples_total = sample_count;
    
    // Load metadata if available
    char meta_path[MAX_PATH_LENGTH];
    strncpy(meta_path, jsonl_path, sizeof(meta_path) - 1);
    meta_path[sizeof(meta_path) - 1] = '\0';
    
    char *jsonl_ext = strstr(meta_path, ".jsonl");
    if (jsonl_ext) {
        strcpy(jsonl_ext, ".meta.json");
        
        FILE *meta_fp = fopen(meta_path, "r");
        if (meta_fp) {
            char meta_buffer[4096];
            size_t bytes_read = fread(meta_buffer, 1, sizeof(meta_buffer) - 1, meta_fp);
            meta_buffer[bytes_read] = '\0';
            fclose(meta_fp);
            
            // Parse metadata JSON
            char config_buf[128], os_buf[128], workload_buf[128];
            parse_json_string(meta_buffer, "config", config_buf, sizeof(config_buf));
            parse_json_string(meta_buffer, "os", os_buf, sizeof(os_buf));
            parse_json_string(meta_buffer, "workload", workload_buf, sizeof(workload_buf));
            
            if (strlen(config_buf) > 0) {
                strncpy(summary->config, config_buf, sizeof(summary->config) - 1);
            }
            if (strlen(os_buf) > 0) {
                strncpy(summary->os, os_buf, sizeof(summary->os) - 1);
            }
            if (strlen(workload_buf) > 0) {
                strncpy(summary->workload, workload_buf, sizeof(summary->workload) - 1);
            }
        }
    }
    
    // Fallback: parse from run ID if metadata not available
    if (strlen(summary->config) == 0) {
        parse_run_id_fallback(summary->run_id, summary->config, summary->workload, summary->os);
    }
    
    // Filter valid samples (reasonable battery percentage and power values)
    int valid_count = 0;
    for (int i = 0; i < sample_count; i++) {
        if (samples[i].percentage >= 0.0 && samples[i].percentage <= 100.0 && 
            samples[i].watts >= 0.0 && samples[i].watts < 100.0) {
            if (valid_count != i) {
                samples[valid_count] = samples[i];
            }
            valid_count++;
        }
    }
    
    if (valid_count < min_samples) {
        free(samples);
        return -1;
    }
    
    summary->samples_valid = valid_count;
    
    // Calculate duration
    summary->duration_s = calculate_duration(samples, valid_count);
    
    // Calculate power statistics
    double *watts_values = malloc(valid_count * sizeof(double));
    if (!watts_values) {
        free(samples);
        return -1;
    }
    
    double watts_sum = 0.0;
    double cpu_sum = 0.0;
    double ram_sum = 0.0;
    double temp_sum = 0.0;
    
    for (int i = 0; i < valid_count; i++) {
        watts_values[i] = samples[i].watts;
        watts_sum += samples[i].watts;
        cpu_sum += samples[i].cpu_load;
        ram_sum += samples[i].ram_pct;
        temp_sum += samples[i].temp_c;
    }
    
    summary->avg_watts = watts_sum / valid_count;
    summary->avg_cpu_load = cpu_sum / valid_count;
    summary->avg_ram_pct = ram_sum / valid_count;
    summary->avg_temp_c = temp_sum / valid_count;
    
    // Sort watts values for percentile calculations
    qsort(watts_values, valid_count, sizeof(double), compare_doubles);
    
    summary->median_watts = percentile(watts_values, valid_count, 0.5);
    summary->p95_watts = percentile(watts_values, valid_count, 0.95);
    
    free(watts_values);
    
    // Calculate battery percentage drop
    if (valid_count >= 2) {
        summary->start_pct = samples[0].percentage;
        summary->end_pct = samples[valid_count - 1].percentage;
        
        if (summary->start_pct > summary->end_pct) {
            summary->pct_drop = summary->start_pct - summary->end_pct;
        }
    }
    
    free(samples);
    return 0;
}

double percentile(double *sorted_data, int count, double p) {
    if (count == 0 || !sorted_data) {
        return 0.0;
    }
    
    if (count == 1) {
        return sorted_data[0];
    }
    
    double index = p * (count - 1);
    int lower_index = (int)floor(index);
    int upper_index = (int)ceil(index);
    
    if (lower_index == upper_index) {
        return sorted_data[lower_index];
    }
    
    double weight = index - lower_index;
    return sorted_data[lower_index] * (1.0 - weight) + sorted_data[upper_index] * weight;
}

static int parse_jsonl_file(const char *jsonl_path, telemetry_sample_t **samples, int *count) {
    FILE *fp = fopen(jsonl_path, "r");
    if (!fp) {
        return -1;
    }
    
    // First pass: count lines
    char line[MAX_LINE_LENGTH];
    int line_count = 0;
    
    while (fgets(line, sizeof(line), fp)) {
        if (strlen(line) > 1) { // Skip empty lines
            line_count++;
        }
    }
    
    if (line_count == 0) {
        fclose(fp);
        return -1;
    }
    
    // Allocate memory for samples
    *samples = malloc(line_count * sizeof(telemetry_sample_t));
    if (!*samples) {
        fclose(fp);
        return -1;
    }
    
    // Second pass: parse lines
    rewind(fp);
    int parsed_count = 0;
    
    while (fgets(line, sizeof(line), fp) && parsed_count < line_count) {
        if (strlen(line) > 1) {
            if (parse_json_line(line, &(*samples)[parsed_count]) == 0) {
                parsed_count++;
            }
        }
    }
    
    fclose(fp);
    *count = parsed_count;
    
    // Shrink array if needed
    if (parsed_count < line_count && parsed_count > 0) {
        *samples = realloc(*samples, parsed_count * sizeof(telemetry_sample_t));
    }
    
    return parsed_count > 0 ? 0 : -1;
}

static int parse_json_line(const char *line, telemetry_sample_t *sample) {
    if (!line || !sample) {
        return -1;
    }
    
    // Parse JSON fields
    char timestamp_buf[64];
    parse_json_string(line, "t", timestamp_buf, sizeof(timestamp_buf));
    strncpy(sample->timestamp, timestamp_buf, sizeof(sample->timestamp) - 1);
    sample->timestamp[sizeof(sample->timestamp) - 1] = '\0';
    
    sample->percentage = parse_json_double(line, "pct");
    sample->watts = parse_json_double(line, "watts");
    sample->cpu_load = parse_json_double(line, "cpu_load");
    sample->ram_pct = parse_json_double(line, "ram_pct");
    sample->temp_c = parse_json_double(line, "temp_c");
    
    char source_buf[32];
    parse_json_string(line, "src", source_buf, sizeof(source_buf));
    strncpy(sample->source, source_buf, sizeof(sample->source) - 1);
    sample->source[sizeof(sample->source) - 1] = '\0';
    
    return 0;
}

static double parse_json_double(const char *json, const char *key) {
    if (!json || !key) {
        return 0.0;
    }
    
    char search_pattern[64];
    snprintf(search_pattern, sizeof(search_pattern), "\"%s\":", key);
    
    const char *key_pos = strstr(json, search_pattern);
    if (!key_pos) {
        return 0.0;
    }
    
    const char *value_start = key_pos + strlen(search_pattern);
    
    // Skip whitespace
    while (*value_start && (*value_start == ' ' || *value_start == '\t')) {
        value_start++;
    }
    
    return strtod(value_start, NULL);
}

static const char *parse_json_string(const char *json, const char *key, char *buffer, size_t buffer_size) {
    if (!json || !key || !buffer || buffer_size == 0) {
        return NULL;
    }
    
    buffer[0] = '\0';
    
    char search_pattern[64];
    snprintf(search_pattern, sizeof(search_pattern), "\"%s\":", key);
    
    const char *key_pos = strstr(json, search_pattern);
    if (!key_pos) {
        return NULL;
    }
    
    const char *value_start = key_pos + strlen(search_pattern);
    
    // Skip whitespace
    while (*value_start && (*value_start == ' ' || *value_start == '\t')) {
        value_start++;
    }
    
    // Expect opening quote
    if (*value_start != '"') {
        return NULL;
    }
    value_start++;
    
    // Find closing quote
    const char *value_end = strchr(value_start, '"');
    if (!value_end) {
        return NULL;
    }
    
    size_t value_len = value_end - value_start;
    if (value_len >= buffer_size) {
        value_len = buffer_size - 1;
    }
    
    strncpy(buffer, value_start, value_len);
    buffer[value_len] = '\0';
    
    return buffer;
}

static int compare_doubles(const void *a, const void *b) {
    double da = *(const double*)a;
    double db = *(const double*)b;
    
    if (da < db) return -1;
    if (da > db) return 1;
    return 0;
}

static void parse_run_id_fallback(const char *run_id, char *config, char *workload, char *os) {
    // Initialize outputs
    strcpy(config, "unknown");
    strcpy(workload, "");
    strcpy(os, "unknown");
    
    if (!run_id) return;
    
    // Parse run ID format: TIMESTAMP_HOSTNAME_OS_CONFIG[_WORKLOAD]
    char *run_copy = strdup(run_id);
    if (!run_copy) return;
    
    char *parts[6];
    int part_count = 0;
    
    char *token = strtok(run_copy, "_");
    while (token && part_count < 6) {
        parts[part_count++] = token;
        token = strtok(NULL, "_");
    }
    
    if (part_count >= 4) {
        // parts[0] = timestamp, parts[1] = hostname, parts[2] = os, parts[3] = config
        strncpy(os, parts[2], 127);
        os[127] = '\0';
        strncpy(config, parts[3], 127);
        config[127] = '\0';
        
        if (part_count >= 5) {
            strncpy(workload, parts[4], 127);
            workload[127] = '\0';
        }
    }
    
    free(run_copy);
}

static double calculate_duration(const telemetry_sample_t *samples, int count) {
    if (count < 2) {
        return 0.0;
    }
    
    // For simplicity, estimate duration based on sample count
    // Assuming roughly 1 sample per minute (default 0.0167 Hz ≈ 1/60 Hz)
    return (double)count * 60.0;
}