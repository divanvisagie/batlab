/*
 * batlab - Battery Test Harness
 * 
 * Cross-platform battery efficiency measurement for FreeBSD vs Linux research.
 * Manual configuration approach - user configures system, tool records data.
 * 
 * This is a C rewrite of the original Rust implementation, maintaining
 * 100% file compatibility with existing reports and data formats.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <getopt.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <signal.h>
#include <time.h>
#include <errno.h>
#include <math.h>
#include <dirent.h>
#include <fcntl.h>
#include <ctype.h>

#ifdef __FreeBSD__
#include <sys/sysctl.h>
#include <sys/types.h>
#endif

#ifdef __linux__
#include <sys/sysinfo.h>
#endif

#include "telemetry.h"

#define VERSION "2.0.0"
// Global state for signal handling
volatile sig_atomic_t running = 1;
FILE *log_file = NULL;

// Function declarations
void signal_handler(int signum);
int cmd_init(const char *project_dir);
int cmd_log(const char *config_name, double hz, const char *output_file, const char *data_dir);
int cmd_run(const char *workload, char *args[], const char *workload_dir);
int cmd_report(const char *data_dir, const char *group_by, const char *format, const char *output_file, const char *baseline, int min_samples);
int cmd_export(const char *data_dir, const char *format, const char *output_file);
int cmd_list(const char *item, const char *workload_dir);
int cmd_sample(void);
int cmd_metadata(void);
int cmd_show_config(void);
void print_usage(void);

// Signal handler for graceful shutdown
void signal_handler(int signum) {
    if (signum == SIGINT || signum == SIGTERM) {
        running = 0;
        if (log_file) {
            fprintf(stderr, "\n⏹️  Received interrupt signal, stopping telemetry...\n");
        }
    }
}

int main(int argc, char *argv[]) {
    // Set up signal handling
    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);
    
    if (argc < 2) {
        print_usage();
        return 1;
    }
    
    // Determine project directory (current working directory)
    char project_dir[MAX_PATH_LENGTH];
    if (!getcwd(project_dir, sizeof(project_dir))) {
        perror("getcwd failed");
        return 1;
    }
    
    char data_dir[MAX_PATH_LENGTH];
    char workload_dir[MAX_PATH_LENGTH];
    snprintf(data_dir, sizeof(data_dir), "%s/data", project_dir);
    snprintf(workload_dir, sizeof(workload_dir), "%s/workload", project_dir);
    
    const char *command = argv[1];
    
    if (strcmp(command, "init") == 0) {
        return cmd_init(project_dir);
    }
    else if (strcmp(command, "log") == 0) {
        const char *config_name = argc > 2 ? argv[2] : NULL;
        double hz = 0.0167; // Default ~1/60 Hz
        const char *output_file = NULL;
        
        // Parse additional arguments
        for (int i = 3; i < argc; i++) {
            if (strcmp(argv[i], "--hz") == 0 && i + 1 < argc) {
                hz = atof(argv[++i]);
            } else if (strcmp(argv[i], "-o") == 0 && i + 1 < argc) {
                output_file = argv[++i];
            } else if (strcmp(argv[i], "--output") == 0 && i + 1 < argc) {
                output_file = argv[++i];
            }
        }
        
        return cmd_log(config_name, hz, output_file, data_dir);
    }
    else if (strcmp(command, "run") == 0) {
        if (argc < 3) {
            fprintf(stderr, "❌ Usage: batlab run <workload> [args...]\n");
            return 1;
        }
        return cmd_run(argv[2], &argv[3], workload_dir);
    }
    else if (strcmp(command, "report") == 0) {
        const char *group_by = "config";
        const char *format = "table";
        const char *output_file = NULL;
        const char *baseline = NULL;
        int min_samples = 10;
        
        // Parse additional arguments
        for (int i = 2; i < argc; i++) {
            if (strcmp(argv[i], "--group-by") == 0 && i + 1 < argc) {
                group_by = argv[++i];
            } else if (strcmp(argv[i], "--format") == 0 && i + 1 < argc) {
                format = argv[++i];
            } else if (strcmp(argv[i], "-o") == 0 && i + 1 < argc) {
                output_file = argv[++i];
            } else if (strcmp(argv[i], "--baseline") == 0 && i + 1 < argc) {
                baseline = argv[++i];
            } else if (strcmp(argv[i], "--min-samples") == 0 && i + 1 < argc) {
                min_samples = atoi(argv[++i]);
            }
        }
        
        return cmd_report(data_dir, group_by, format, output_file, baseline, min_samples);
    }
    else if (strcmp(command, "export") == 0) {
        const char *format = "csv";
        const char *output_file = NULL;
        
        // Parse additional arguments
        for (int i = 2; i < argc; i++) {
            if (strcmp(argv[i], "--format") == 0 && i + 1 < argc) {
                format = argv[++i];
            } else if (strcmp(argv[i], "-o") == 0 && i + 1 < argc) {
                output_file = argv[++i];
            }
        }
        
        return cmd_export(data_dir, format, output_file);
    }
    else if (strcmp(command, "list") == 0) {
        const char *item = argc > 2 ? argv[2] : "workloads";
        return cmd_list(item, workload_dir);
    }
    else if (strcmp(command, "sample") == 0) {
        return cmd_sample();
    }
    else if (strcmp(command, "metadata") == 0) {
        return cmd_metadata();
    }
    else if (strcmp(command, "show-config") == 0) {
        return cmd_show_config();
    }
    else {
        fprintf(stderr, "❌ Unknown command: %s\n", command);
        print_usage();
        return 1;
    }
}

void print_usage(void) {
    printf("batlab %s - Battery Test Harness for FreeBSD vs Linux Research\n\n", VERSION);
    printf("USAGE:\n");
    printf("    batlab <COMMAND>\n\n");
    printf("COMMANDS:\n");
    printf("    init                           Initialize directories and check system capabilities\n");
    printf("    log [CONFIG-NAME]              Start telemetry logging with optional configuration name\n");
    printf("    run <WORKLOAD> [ARGS...]       Run workload (use in separate terminal while logging)\n");
    printf("    report [OPTIONS]               Analyze collected data and display results\n");
    printf("    export [OPTIONS]               Export summary data for external analysis\n");
    printf("    list [workloads]               List available workloads\n");
    printf("    sample                         Collect a single telemetry sample (for testing)\n");
    printf("    metadata                       Show system metadata\n");
    printf("    show-config                    Show what auto-generated config name would be used\n\n");
    printf("EXAMPLES:\n");
    printf("    batlab init                    # Set up directories and example workloads\n");
    printf("    batlab show-config             # Preview auto-generated config name\n");
    printf("    batlab log                     # Start logging with auto-generated config name\n");
    printf("    batlab log freebsd-powerd      # Start logging with custom config name\n");
    printf("    batlab run idle                # Run idle workload in separate terminal\n");
    printf("    batlab report                  # View results\n");
    printf("    batlab list workloads          # Show available workloads\n\n");
    printf("For more information, see README.md\n");
}

int cmd_init(const char *project_dir) {
    printf("🔋 Initializing batlab battery test harness...\n");
    
    // Create directories
    char data_dir[MAX_PATH_LENGTH];
    char workload_dir[MAX_PATH_LENGTH];
    char report_dir[MAX_PATH_LENGTH];
    
    snprintf(data_dir, sizeof(data_dir), "%s/data", project_dir);
    snprintf(workload_dir, sizeof(workload_dir), "%s/workload", project_dir);
    snprintf(report_dir, sizeof(report_dir), "%s/report", project_dir);
    
    create_directory(data_dir);
    create_directory(workload_dir);
    create_directory(report_dir);
    
    // Create example workloads
    if (create_example_workloads(workload_dir) != 0) {
        fprintf(stderr, "⚠️  Warning: Failed to create example workloads\n");
    }
    
    // Detect OS and capabilities
    printf("🔍 Detecting system capabilities...\n");
    
    char hostname[256], os[256], kernel[256], cpu[256], machine[256];
    if (get_system_info(hostname, os, kernel, cpu, machine) == 0) {
        printf("💻 Detected: %s system\n", os);
    }
    
    // Check battery access
    double dummy_pct, dummy_watts;
    char dummy_source[32];
    if (get_battery_info(&dummy_pct, &dummy_watts, dummy_source) == 0) {
        printf("✅ Battery telemetry available via %s\n", dummy_source);
    } else {
        printf("⚠️  Battery telemetry not available - check system setup\n");
    }
    
    printf("✅ Initialization complete!\n");
    printf("📋 Next steps:\n");
    printf("   1. Manually configure your system power management\n");
    printf("   2. Run: batlab log (auto-detects config) or batlab log <config-name> (in terminal 1)\n");
    printf("   3. Run: batlab run <workload> (in terminal 2)\n");
    
    return 0;
}

int cmd_log(const char *config_name, double hz, const char *output_file, const char *data_dir) {
    char actual_config[256];
    
    // Generate or validate config name
    if (config_name) {
        strncpy(actual_config, config_name, sizeof(actual_config) - 1);
        actual_config[sizeof(actual_config) - 1] = '\0';
    } else {
        if (generate_auto_config_name(actual_config, sizeof(actual_config)) != 0) {
            fprintf(stderr, "❌ Failed to auto-generate config name\n");
            fprintf(stderr, "💡 Please provide a config name manually: batlab log <config-name>\n");
            return 1;
        }
        printf("🤖 Auto-generated config name: %s\n", actual_config);
    }
    
    // Validate frequency
    if (hz < 0.01 || hz > 10.0) {
        fprintf(stderr, "❌ Sampling frequency must be between 0.01 and 10.0 Hz\n");
        return 1;
    }
    
    // Wait for battery to be ready
    if (wait_for_battery_ready() != 0) {
        return 1;
    }
    
    // Create data directory if needed
    create_directory(data_dir);
    
    // Generate run ID and file paths
    char run_id[512];
    generate_run_id(actual_config, NULL, run_id, sizeof(run_id));
    
    char jsonl_file[MAX_PATH_LENGTH];
    char meta_file[MAX_PATH_LENGTH];
    
    if (output_file) {
        strncpy(jsonl_file, output_file, sizeof(jsonl_file) - 1);
        jsonl_file[sizeof(jsonl_file) - 1] = '\0';
        snprintf(meta_file, sizeof(meta_file), "%s.meta.json", output_file);
    } else {
        snprintf(jsonl_file, sizeof(jsonl_file), "%s/%s.jsonl", data_dir, run_id);
        snprintf(meta_file, sizeof(meta_file), "%s/%s.meta.json", data_dir, run_id);
    }
    
    printf("🔋 Starting telemetry logging...\n");
    printf("⚙️  Configuration: %s\n", actual_config);
    printf("📊 Run ID: %s\n", run_id);
    printf("📁 Output: %s\n", jsonl_file);
    printf("🔄 Sampling at %.1f Hz\n", hz);
    printf("⏹️  Press Ctrl+C to stop logging\n");
    
    // Create metadata file
    char hostname[256], os[256], kernel[256], cpu[256], machine[256];
    get_system_info(hostname, os, kernel, cpu, machine);
    
    char timestamp[64];
    get_current_timestamp(timestamp, sizeof(timestamp));
    
    FILE *meta_fp = fopen(meta_file, "w");
    if (meta_fp) {
        fprintf(meta_fp, "{\n");
        fprintf(meta_fp, "  \"run_id\": \"%s\",\n", run_id);
        fprintf(meta_fp, "  \"host\": \"%s\",\n", hostname);
        fprintf(meta_fp, "  \"os\": \"%s\",\n", os);
        fprintf(meta_fp, "  \"config\": \"%s\",\n", actual_config);
        fprintf(meta_fp, "  \"start_time\": \"%s\",\n", timestamp);
        fprintf(meta_fp, "  \"sampling_hz\": %f\n", hz);
        fprintf(meta_fp, "}\n");
        fclose(meta_fp);
    }
    
    // Open log file
    log_file = fopen(jsonl_file, "w");
    if (!log_file) {
        perror("Failed to open log file");
        return 1;
    }
    
    // Calculate sleep duration
    double sleep_duration = 1.0 / hz;
    unsigned long sleep_usec = (unsigned long)(sleep_duration * 1000000);
    
    int sample_count = 0;
    int error_count = 0;
    
    printf("🚀 Logging started - run workload in another terminal\n");
    
    // Prevent system suspension
    prevent_system_suspension();
    
    // Main sampling loop
    while (running) {
        telemetry_sample_t sample;
        if (collect_telemetry(&sample) == 0) {
            // Write JSON line
            fprintf(log_file, "{\"t\": \"%s\", \"pct\": %.1f, \"watts\": %.3f, "
                   "\"cpu_load\": %.2f, \"ram_pct\": %.3f, \"temp_c\": %.2f, \"src\": \"%s\"}\n",
                   sample.timestamp, sample.percentage, sample.watts,
                   sample.cpu_load, sample.ram_pct, sample.temp_c, sample.source);
            
            sample_count++;
            
            // Flush periodically
            if (sample_count % 10 == 0) {
                fflush(log_file);
            }
        } else {
            error_count++;
            fprintf(stderr, "⚠️  Warning: Telemetry collection failed\n");
            
            // Exit if too many errors during startup
            if (error_count > 10 && sample_count == 0) {
                fprintf(stderr, "❌ Too many failures during startup, exiting\n");
                break;
            }
        }
        
        usleep(sleep_usec);
    }
    
    // Cleanup
    if (log_file) {
        fclose(log_file);
        log_file = NULL;
    }
    
    restore_system_suspension();
    
    printf("\n📊 Telemetry logging stopped\n");
    printf("📈 Samples collected: %d\n", sample_count);
    if (error_count > 0) {
        printf("⚠️  Errors encountered: %d\n", error_count);
    }
    
    return 0;
}

int cmd_run(const char *workload, char *args[] __attribute__((unused)), const char *workload_dir) {
    char workload_file[MAX_PATH_LENGTH];
    snprintf(workload_file, sizeof(workload_file), "%s/%s.sh", workload_dir, workload);
    
    if (!file_exists(workload_file)) {
        fprintf(stderr, "❌ Workload not found: %s\n", workload_file);
        fprintf(stderr, "📋 Available workloads:\n");
        cmd_list("workloads", workload_dir);
        return 1;
    }
    
    printf("🏃 Running workload: %s\n", workload);
    
    // Prevent system suspension during workload
    prevent_system_suspension();
    
    // Execute the workload script
    pid_t pid = fork();
    if (pid == 0) {
        // Child process - execute workload
        execl("/bin/sh", "sh", workload_file, (char *)NULL);
        perror("execl failed");
        exit(1);
    } else if (pid > 0) {
        // Parent process - wait for completion
        int status;
        waitpid(pid, &status, 0);
        
        restore_system_suspension();
        
        if (WIFEXITED(status) && WEXITSTATUS(status) == 0) {
            printf("✅ Workload completed successfully\n");
            return 0;
        } else {
            fprintf(stderr, "❌ Workload failed with exit code: %d\n", WEXITSTATUS(status));
            return 1;
        }
    } else {
        perror("fork failed");
        restore_system_suspension();
        return 1;
    }
}

int cmd_sample(void) {
    if (wait_for_battery_ready() != 0) {
        return 1;
    }
    
    telemetry_sample_t sample;
    if (collect_telemetry(&sample) == 0) {
        printf("{\n");
        printf("  \"t\": \"%s\",\n", sample.timestamp);
        printf("  \"pct\": %.1f,\n", sample.percentage);
        printf("  \"watts\": %.3f,\n", sample.watts);
        printf("  \"cpu_load\": %.2f,\n", sample.cpu_load);
        printf("  \"ram_pct\": %.3f,\n", sample.ram_pct);
        printf("  \"temp_c\": %.2f,\n", sample.temp_c);
        printf("  \"src\": \"%s\"\n", sample.source);
        printf("}\n");
        return 0;
    } else {
        fprintf(stderr, "❌ Telemetry collection failed\n");
        return 1;
    }
}

int cmd_metadata(void) {
    char hostname[256], os[256], kernel[256], cpu[256], machine[256];
    if (get_system_info(hostname, os, kernel, cpu, machine) == 0) {
        printf("{\n");
        printf("  \"hostname\": \"%s\",\n", hostname);
        printf("  \"os\": \"%s\",\n", os);
        printf("  \"kernel\": \"%s\",\n", kernel);
        printf("  \"cpu\": \"%s\",\n", cpu);
        printf("  \"machine\": \"%s\"\n", machine);
        printf("}\n");
        return 0;
    } else {
        fprintf(stderr, "❌ Failed to get system information\n");
        return 1;
    }
}

int cmd_show_config(void) {
    printf("🔍 Detecting system configuration...\n");
    
    char hostname[256], os[256], kernel[256], cpu[256], machine[256];
    if (get_system_info(hostname, os, kernel, cpu, machine) == 0) {
        printf("💻 Operating System: %s\n", os);
        printf("🏠 Hostname: %s\n", hostname);
        printf("⚙️  CPU: %s\n", cpu);
        printf("🖥️  Machine: %s\n", machine);
    }
    
    char config_name[256];
    if (generate_auto_config_name(config_name, sizeof(config_name)) == 0) {
        printf("\n🤖 Auto-generated config name: %s\n", config_name);
        printf("💡 This name is based on your OS and hardware configuration\n");
        printf("📋 Use this with: batlab log %s\n", config_name);
        printf("🔄 Or just run: batlab log (auto-detects)\n");
        return 0;
    } else {
        fprintf(stderr, "❌ Failed to generate config name\n");
        fprintf(stderr, "💡 You may need to provide a config name manually\n");
        return 1;
    }
}

int cmd_list(const char *item, const char *workload_dir) {
    if (strcmp(item, "workloads") == 0) {
        printf("📋 Available workloads:\n");
        
        DIR *dir = opendir(workload_dir);
        if (!dir) {
            printf("⚠️  No workloads directory found\n");
            printf("💡 Run 'batlab init' to create example workloads\n");
            return 0;
        }
        
        struct dirent *entry;
        while ((entry = readdir(dir)) != NULL) {
            if (strstr(entry->d_name, ".sh")) {
                char *name = strdup(entry->d_name);
                char *dot = strrchr(name, '.');
                if (dot) *dot = '\0';
                
                char workload_path[MAX_PATH_LENGTH];
                snprintf(workload_path, sizeof(workload_path), "%s/%s", workload_dir, entry->d_name);
                
                printf("  📄 %-20s ", name);
                
                // Try to get description from workload file
                FILE *fp = fopen(workload_path, "r");
                if (fp) {
                    char line[MAX_LINE_LENGTH];
                    int found_desc = 0;
                    for (int i = 0; i < 10 && fgets(line, sizeof(line), fp); i++) {
                        if (strncmp(line, "# ", 2) == 0 && strncmp(line, "#!/", 3) != 0) {
                            printf("%s", line + 2);
                            found_desc = 1;
                            break;
                        }
                    }
                    if (!found_desc) {
                        printf("No description\n");
                    }
                    fclose(fp);
                } else {
                    printf("No description\n");
                }
                
                free(name);
            }
        }
        closedir(dir);
        return 0;
    } else {
        fprintf(stderr, "❌ Usage: batlab list workloads\n");
        return 1;
    }
}

int cmd_report(const char *data_dir, const char *group_by __attribute__((unused)), const char *format __attribute__((unused)), 
               const char *output_file __attribute__((unused)), const char *baseline __attribute__((unused)), int min_samples) {
    run_summary_t *summaries = NULL;
    int count = 0;
    
    if (load_run_summaries(data_dir, min_samples, &summaries, &count) != 0) {
        fprintf(stderr, "❌ Failed to load run summaries\n");
        return 1;
    }
    
    if (count == 0) {
        fprintf(stderr, "❌ No valid runs found in %s\n", data_dir);
        fprintf(stderr, "💡 Make sure you have collected some telemetry data first:\n");
        fprintf(stderr, "   batlab log <config-name>\n");
        return 0;
    }
    
    // Generate simple table report for now
    printf("INDIVIDUAL RUNS\n");
    printf("%-30s %-15s %-10s %-10s %-8s %-8s %-8s %-8s %-8s\n",
           "RUN_ID", "CONFIG", "OS", "WORKLOAD", "SAMPLES", "AVG_W", "MED_W", "CPU%", "TEMP°C");
    for (int i = 0; i < 120; i++) printf("-");
    printf("\n");
    
    for (int i = 0; i < count; i++) {
        run_summary_t *s = &summaries[i];
        char short_run_id[31];
        strncpy(short_run_id, s->run_id, 30);
        short_run_id[30] = '\0';
        
        char short_config[16];
        strncpy(short_config, s->config, 15);
        short_config[15] = '\0';
        
        char short_os[11];
        strncpy(short_os, s->os, 10);
        short_os[10] = '\0';
        
        printf("%-30s %-15s %-10s %-10s %-8d %-8.2f %-8.2f %-8.1f %-8.1f\n",
               short_run_id, short_config, short_os, 
               strlen(s->workload) > 0 ? s->workload : "-",
               s->samples_valid, s->avg_watts, s->median_watts,
               s->avg_cpu_load * 100.0, s->avg_temp_c);
    }
    
    free(summaries);
    return 0;
}

int cmd_export(const char *data_dir, const char *format, const char *output_file) {
    return cmd_report(data_dir, "config", format, output_file, NULL, 1);
}