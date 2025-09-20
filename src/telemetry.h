/*
 * telemetry.h - Header file for batlab telemetry collection
 * 
 * Cross-platform telemetry collection for FreeBSD and Linux systems.
 * Provides battery, CPU, memory, and temperature monitoring.
 */

#ifndef TELEMETRY_H
#define TELEMETRY_H

#include <stdio.h>
#include <time.h>

#define MAX_LINE_LENGTH 1024
#define MAX_PATH_LENGTH 512

// Data structures
typedef struct {
    char timestamp[64];
    double percentage;
    double watts;
    double cpu_load;
    double ram_pct;
    double temp_c;
    char source[32];
} telemetry_sample_t;

typedef struct {
    char run_id[256];
    char config[128];
    char os[128];
    char workload[128];
    double duration_s;
    int samples_total;
    int samples_valid;
    double avg_watts;
    double median_watts;
    double p95_watts;
    double avg_cpu_load;
    double avg_ram_pct;
    double avg_temp_c;
    double pct_drop;
    double start_pct;
    double end_pct;
} run_summary_t;

// Core telemetry functions
int collect_telemetry(telemetry_sample_t *sample);
int get_battery_info(double *percentage, double *watts, char *source);
int get_system_metrics(double *cpu_load, double *ram_pct, double *temp_c);
int get_system_info(char *hostname, char *os, char *kernel, char *cpu, char *machine);

// Configuration and identification
int generate_auto_config_name(char *config_name, size_t len);
void generate_run_id(const char *config, const char *workload, char *run_id, size_t len);

// Battery management
int wait_for_battery_ready(void);

// System power management
int prevent_system_suspension(void);
void restore_system_suspension(void);

// Analysis functions
int load_run_summaries(const char *data_dir, int min_samples, run_summary_t **summaries, int *count);
int analyze_run(const char *jsonl_path, int min_samples, run_summary_t *summary);
double percentile(double *sorted_data, int count, double p);

// Utility functions
void create_directory(const char *path);
int file_exists(const char *path);
void get_current_timestamp(char *timestamp, size_t len);
double get_current_time(void);
int create_example_workloads(const char *workload_dir);

#endif /* TELEMETRY_H */