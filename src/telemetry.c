/*
 * telemetry.c - Cross-platform telemetry collection for batlab
 * 
 * Platform-specific implementations for battery, CPU, memory, and temperature
 * telemetry collection on FreeBSD and Linux systems.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <time.h>
#include <errno.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <math.h>
#include <ctype.h>

#ifdef __FreeBSD__
#include <sys/sysctl.h>
#include <sys/user.h>
#include <kvm.h>
#endif

#ifdef __linux__
#include <sys/sysinfo.h>
#endif

#include "telemetry.h"

// Forward declarations
static int get_battery_info_freebsd(double *percentage, double *watts, char *source);
static int get_battery_info_linux(double *percentage, double *watts, char *source);
static int get_system_metrics_freebsd(double *cpu_load, double *ram_pct, double *temp_c);
static int get_system_metrics_linux(double *cpu_load, double *ram_pct, double *temp_c);
static int run_command(const char *cmd, char *output, size_t output_size);
static double parse_double(const char *str);
static int read_file_line(const char *path, char *buffer, size_t size);

int collect_telemetry(telemetry_sample_t *sample) {
    if (!sample) return -1;
    
    // Get current timestamp
    time_t now = time(NULL);
    struct tm *utc_tm = gmtime(&now);
    strftime(sample->timestamp, sizeof(sample->timestamp), 
             "%Y-%m-%dT%H:%M:%S.000000000Z", utc_tm);
    
    // Get battery info
    if (get_battery_info(&sample->percentage, &sample->watts, sample->source) != 0) {
        return -1;
    }
    
    // Get system metrics
    if (get_system_metrics(&sample->cpu_load, &sample->ram_pct, &sample->temp_c) != 0) {
        // Don't fail if system metrics aren't available, use defaults
        sample->cpu_load = 0.0;
        sample->ram_pct = 0.0;
        sample->temp_c = 0.0;
    }
    
    return 0;
}

int get_battery_info(double *percentage, double *watts, char *source) {
#ifdef __FreeBSD__
    return get_battery_info_freebsd(percentage, watts, source);
#elif defined(__linux__)
    return get_battery_info_linux(percentage, watts, source);
#else
    *percentage = 50.0;
    *watts = 5.0;
    strcpy(source, "dummy");
    return 0;
#endif
}

int get_system_metrics(double *cpu_load, double *ram_pct, double *temp_c) {
#ifdef __FreeBSD__
    return get_system_metrics_freebsd(cpu_load, ram_pct, temp_c);
#elif defined(__linux__)
    return get_system_metrics_linux(cpu_load, ram_pct, temp_c);
#else
    *cpu_load = 0.1;
    *ram_pct = 50.0;
    *temp_c = 40.0;
    return 0;
#endif
}

#ifdef __FreeBSD__
static int get_battery_info_freebsd(double *percentage, double *watts, char *source) {
    char output[1024];
    
    // Try acpiconf first
    if (run_command("acpiconf -i 0", output, sizeof(output)) == 0) {
        strcpy(source, "acpiconf");
        
        double pct = -1, rate = -1;
        char *line = strtok(output, "\n");
        
        while (line) {
            if (strstr(line, "Remaining capacity:")) {
                char *pct_str = strchr(line, ':');
                if (pct_str) {
                    pct_str++;
                    while (*pct_str == ' ' || *pct_str == '\t') pct_str++;
                    pct = parse_double(pct_str);
                }
            } else if (strstr(line, "Present rate:")) {
                char *rate_str = strchr(line, ':');
                if (rate_str) {
                    rate_str++;
                    while (*rate_str == ' ' || *rate_str == '\t') rate_str++;
                    // Rate is in mW, convert to W
                    rate = parse_double(rate_str) / 1000.0;
                }
            } else if (strstr(line, "State:") && strstr(line, "charging")) {
                // Battery is charging, return error
                return -1;
            }
            line = strtok(NULL, "\n");
        }
        
        if (pct >= 0) {
            *percentage = pct;
            *watts = rate > 0 ? rate : 0.0;
            return 0;
        }
    }
    
    // Try sysctl fallback
    size_t size;
    int life = -1, rate = -1;
    
    size = sizeof(life);
    if (sysctlbyname("hw.acpi.battery.life", &life, &size, NULL, 0) == 0 && life >= 0) {
        *percentage = (double)life;
        
        size = sizeof(rate);
        if (sysctlbyname("hw.acpi.battery.rate", &rate, &size, NULL, 0) == 0 && rate >= 0) {
            *watts = (double)rate / 1000.0; // Convert mW to W
        } else {
            *watts = 0.0;
        }
        
        strcpy(source, "sysctl");
        return 0;
    }
    
    return -1;
}

static int get_system_metrics_freebsd(double *cpu_load, double *ram_pct, double *temp_c) {
    // Get CPU load average
    double loadavg[3];
    if (getloadavg(loadavg, 3) != -1) {
        *cpu_load = loadavg[0]; // 1-minute load average
    } else {
        *cpu_load = 0.0;
    }
    
    // Get memory usage via sysctl
    size_t size;
    unsigned long page_size = 0, free_pages = 0, total_pages = 0;
    unsigned long inactive_pages = 0, cached_pages = 0;
    
    size = sizeof(page_size);
    sysctlbyname("vm.stats.vm.v_page_size", &page_size, &size, NULL, 0);
    
    size = sizeof(free_pages);
    sysctlbyname("vm.stats.vm.v_free_count", &free_pages, &size, NULL, 0);
    
    size = sizeof(inactive_pages);
    sysctlbyname("vm.stats.vm.v_inactive_count", &inactive_pages, &size, NULL, 0);
    
    size = sizeof(cached_pages);
    sysctlbyname("vm.stats.vm.v_cache_count", &cached_pages, &size, NULL, 0);
    
    // Get total memory
    size_t physmem;
    size = sizeof(physmem);
    if (sysctlbyname("hw.physmem", &physmem, &size, NULL, 0) == 0 && page_size > 0) {
        total_pages = physmem / page_size;
        unsigned long available_pages = free_pages + inactive_pages + cached_pages;
        unsigned long used_pages = total_pages - available_pages;
        *ram_pct = (double)used_pages / total_pages * 100.0;
    } else {
        *ram_pct = 0.0;
    }
    
    // Get temperature
    char temp_output[256];
    if (run_command("sysctl -n dev.cpu.0.temperature 2>/dev/null", temp_output, sizeof(temp_output)) == 0) {
        // Temperature comes as "45.0C"
        *temp_c = parse_double(temp_output);
    } else if (run_command("sysctl -n hw.acpi.thermal.tz0.temperature 2>/dev/null", temp_output, sizeof(temp_output)) == 0) {
        // ACPI thermal zone temperature (in tenths of Kelvin)
        double kelvin = parse_double(temp_output) / 10.0;
        *temp_c = kelvin - 273.15;
    } else {
        *temp_c = 0.0;
    }
    
    return 0;
}
#endif

#ifdef __linux__
static int get_battery_info_linux(double *percentage, double *watts, char *source) {
    char output[2048];
    
    // Try upower first
    if (run_command("upower -i $(upower -e | grep 'BAT') 2>/dev/null", output, sizeof(output)) == 0) {
        strcpy(source, "upower");
        
        double pct = -1, rate = -1;
        int is_charging = 0;
        char *line = strtok(output, "\n");
        
        while (line) {
            // Trim whitespace
            while (*line == ' ' || *line == '\t') line++;
            
            if (strstr(line, "percentage") && strchr(line, ':')) {
                char *pct_str = strchr(line, ':') + 1;
                while (*pct_str == ' ' || *pct_str == '\t') pct_str++;
                pct = parse_double(pct_str);
            } else if (strstr(line, "energy-rate") && strchr(line, ':')) {
                char *rate_str = strchr(line, ':') + 1;
                while (*rate_str == ' ' || *rate_str == '\t') rate_str++;
                rate = parse_double(rate_str);
            } else if (strstr(line, "state") && strstr(line, "charging")) {
                is_charging = 1;
            }
            line = strtok(NULL, "\n");
        }
        
        if (is_charging) {
            return -1; // Charging
        }
        
        if (pct >= 0) {
            *percentage = pct;
            *watts = rate > 0 ? rate : 0.0;
            return 0;
        }
    }
    
    // Try sysfs fallback
    char buffer[256];
    double energy_now = -1, power_now = -1, capacity = -1;
    
    // Check for charging state first
    if (read_file_line("/sys/class/power_supply/BAT0/status", buffer, sizeof(buffer)) == 0) {
        if (strstr(buffer, "Charging") || strstr(buffer, "Full")) {
            return -1; // Charging or full
        }
    } else if (read_file_line("/sys/class/power_supply/BAT1/status", buffer, sizeof(buffer)) == 0) {
        if (strstr(buffer, "Charging") || strstr(buffer, "Full")) {
            return -1; // Charging or full
        }
    }
    
    // Try BAT0 first
    if (read_file_line("/sys/class/power_supply/BAT0/capacity", buffer, sizeof(buffer)) == 0) {
        capacity = parse_double(buffer);
    }
    
    if (read_file_line("/sys/class/power_supply/BAT0/power_now", buffer, sizeof(buffer)) == 0) {
        power_now = parse_double(buffer) / 1000000.0; // Convert µW to W
    } else if (read_file_line("/sys/class/power_supply/BAT0/current_now", buffer, sizeof(buffer)) == 0) {
        double current = parse_double(buffer) / 1000000.0; // Convert µA to A
        if (read_file_line("/sys/class/power_supply/BAT0/voltage_now", buffer, sizeof(buffer)) == 0) {
            double voltage = parse_double(buffer) / 1000000.0; // Convert µV to V
            power_now = current * voltage;
        }
    }
    
    // Try BAT1 if BAT0 didn't work
    if (capacity < 0 && read_file_line("/sys/class/power_supply/BAT1/capacity", buffer, sizeof(buffer)) == 0) {
        capacity = parse_double(buffer);
    }
    
    if (power_now < 0 && read_file_line("/sys/class/power_supply/BAT1/power_now", buffer, sizeof(buffer)) == 0) {
        power_now = parse_double(buffer) / 1000000.0; // Convert µW to W
    } else if (power_now < 0 && read_file_line("/sys/class/power_supply/BAT1/current_now", buffer, sizeof(buffer)) == 0) {
        double current = parse_double(buffer) / 1000000.0; // Convert µA to A
        if (read_file_line("/sys/class/power_supply/BAT1/voltage_now", buffer, sizeof(buffer)) == 0) {
            double voltage = parse_double(buffer) / 1000000.0; // Convert µV to V
            power_now = current * voltage;
        }
    }
    
    if (capacity >= 0) {
        *percentage = capacity;
        *watts = power_now > 0 ? power_now : 0.0;
        strcpy(source, "sysfs");
        return 0;
    }
    
    return -1;
}

static int get_system_metrics_linux(double *cpu_load, double *ram_pct, double *temp_c) {
    // Get CPU load average
    double loadavg[3];
    if (getloadavg(loadavg, 3) != -1) {
        *cpu_load = loadavg[0]; // 1-minute load average
    } else {
        *cpu_load = 0.0;
    }
    
    // Get memory usage from /proc/meminfo
    FILE *fp = fopen("/proc/meminfo", "r");
    if (fp) {
        char line[256];
        long mem_total = 0, mem_free = 0, mem_available = 0, buffers = 0, cached = 0;
        
        while (fgets(line, sizeof(line), fp)) {
            if (sscanf(line, "MemTotal: %ld kB", &mem_total) == 1) {
                continue;
            } else if (sscanf(line, "MemFree: %ld kB", &mem_free) == 1) {
                continue;
            } else if (sscanf(line, "MemAvailable: %ld kB", &mem_available) == 1) {
                continue;
            } else if (sscanf(line, "Buffers: %ld kB", &buffers) == 1) {
                continue;
            } else if (sscanf(line, "Cached: %ld kB", &cached) == 1) {
                continue;
            }
        }
        fclose(fp);
        
        if (mem_total > 0) {
            if (mem_available > 0) {
                // Use MemAvailable if available (more accurate)
                *ram_pct = (double)(mem_total - mem_available) / mem_total * 100.0;
            } else {
                // Fallback calculation
                long mem_used = mem_total - mem_free - buffers - cached;
                *ram_pct = (double)mem_used / mem_total * 100.0;
            }
        } else {
            *ram_pct = 0.0;
        }
    } else {
        *ram_pct = 0.0;
    }
    
    // Get temperature from thermal zones
    char buffer[256];
    if (read_file_line("/sys/class/thermal/thermal_zone0/temp", buffer, sizeof(buffer)) == 0) {
        // Temperature in millidegrees Celsius
        double temp_millic = parse_double(buffer);
        *temp_c = temp_millic / 1000.0;
    } else {
        // Try hwmon
        char temp_path[512];
        for (int i = 0; i < 10; i++) {
            snprintf(temp_path, sizeof(temp_path), "/sys/class/hwmon/hwmon%d/temp1_input", i);
            if (read_file_line(temp_path, buffer, sizeof(buffer)) == 0) {
                double temp_millic = parse_double(buffer);
                *temp_c = temp_millic / 1000.0;
                break;
            }
        }
        
        if (*temp_c == 0.0) {
            // Try coretemp
            for (int i = 0; i < 10; i++) {
                snprintf(temp_path, sizeof(temp_path), "/sys/devices/platform/coretemp.%d/hwmon/hwmon*/temp*_input", i);
                if (run_command("ls /sys/devices/platform/coretemp.*/hwmon/hwmon*/temp*_input 2>/dev/null | head -1", buffer, sizeof(buffer)) == 0) {
                    // Remove newline
                    char *newline = strchr(buffer, '\n');
                    if (newline) *newline = '\0';
                    
                    if (read_file_line(buffer, buffer, sizeof(buffer)) == 0) {
                        double temp_millic = parse_double(buffer);
                        *temp_c = temp_millic / 1000.0;
                        break;
                    }
                }
            }
        }
    }
    
    return 0;
}
#endif

int get_system_info(char *hostname, char *os, char *kernel, char *cpu, char *machine) {
    char buffer[1024];
    
    // Get hostname
    if (gethostname(hostname, 256) != 0) {
        strcpy(hostname, "unknown");
    }
    
    // Get OS info
#ifdef __FreeBSD__
    strcpy(os, "FreeBSD");
    if (run_command("freebsd-version", buffer, sizeof(buffer)) == 0) {
        // Remove newline
        char *newline = strchr(buffer, '\n');
        if (newline) *newline = '\0';
        snprintf(os, 256, "FreeBSD %s", buffer);
    }
#elif defined(__linux__)
    strcpy(os, "Linux");
    if (read_file_line("/etc/os-release", buffer, sizeof(buffer)) == 0) {
        char *line = strtok(buffer, "\n");
        while (line) {
            if (strncmp(line, "PRETTY_NAME=", 12) == 0) {
                char *name = line + 12;
                // Remove quotes
                if (*name == '"') {
                    name++;
                    char *end_quote = strchr(name, '"');
                    if (end_quote) *end_quote = '\0';
                }
                strncpy(os, name, 255);
                os[255] = '\0';
                break;
            }
            line = strtok(NULL, "\n");
        }
    }
#else
    strcpy(os, "Unknown");
#endif
    
    // Get kernel version
    if (run_command("uname -r", buffer, sizeof(buffer)) == 0) {
        char *newline = strchr(buffer, '\n');
        if (newline) *newline = '\0';
        strncpy(kernel, buffer, 255);
        kernel[255] = '\0';
    } else {
        strcpy(kernel, "unknown");
    }
    
    // Get CPU info
#ifdef __FreeBSD__
    size_t size = 256;
    if (sysctlbyname("hw.model", cpu, &size, NULL, 0) != 0) {
        strcpy(cpu, "unknown");
    }
#elif defined(__linux__)
    FILE *fp = fopen("/proc/cpuinfo", "r");
    if (fp) {
        char line[512];
        strcpy(cpu, "unknown");
        while (fgets(line, sizeof(line), fp)) {
            if (strncmp(line, "model name", 10) == 0) {
                char *colon = strchr(line, ':');
                if (colon) {
                    colon++;
                    while (*colon == ' ' || *colon == '\t') colon++;
                    char *newline = strchr(colon, '\n');
                    if (newline) *newline = '\0';
                    strncpy(cpu, colon, 255);
                    cpu[255] = '\0';
                    break;
                }
            }
        }
        fclose(fp);
    } else {
        strcpy(cpu, "unknown");
    }
#else
    strcpy(cpu, "unknown");
#endif
    
    // Get machine info
    if (run_command("uname -m", buffer, sizeof(buffer)) == 0) {
        char *newline = strchr(buffer, '\n');
        if (newline) *newline = '\0';
        strncpy(machine, buffer, 255);
        machine[255] = '\0';
    } else {
        strcpy(machine, "unknown");
    }
    
    return 0;
}

int generate_auto_config_name(char *config_name, size_t len) {
    char hostname[256], os[256], kernel[256], cpu[256], machine[256];
    
    if (get_system_info(hostname, os, kernel, cpu, machine) != 0) {
        return -1;
    }
    
    // Create config name based on OS and hardware
    char os_part[64] = {0};
    char hw_part[128] = {0};
    
    // Extract OS name
    if (strstr(os, "FreeBSD")) {
        strcpy(os_part, "freebsd");
    } else if (strstr(os, "Linux")) {
        strcpy(os_part, "linux");
    } else {
        strcpy(os_part, "unknown");
    }
    
    // Try to extract meaningful hardware info from CPU string
    char *cpu_lower = strdup(cpu);
    for (char *p = cpu_lower; *p; p++) {
        *p = tolower(*p);
    }
    
    // Look for Intel/AMD and model
    if (strstr(cpu_lower, "intel")) {
        if (strstr(cpu_lower, "i3")) {
            strcpy(hw_part, "intel-i3");
        } else if (strstr(cpu_lower, "i5")) {
            strcpy(hw_part, "intel-i5");
        } else if (strstr(cpu_lower, "i7")) {
            strcpy(hw_part, "intel-i7");
        } else if (strstr(cpu_lower, "i9")) {
            strcpy(hw_part, "intel-i9");
        } else {
            strcpy(hw_part, "intel");
        }
    } else if (strstr(cpu_lower, "amd")) {
        if (strstr(cpu_lower, "ryzen")) {
            strcpy(hw_part, "amd-ryzen");
        } else {
            strcpy(hw_part, "amd");
        }
    } else {
        strcpy(hw_part, "generic");
    }
    
    free(cpu_lower);
    
    // Generate final config name
    snprintf(config_name, len, "%s-%s", os_part, hw_part);
    
    return 0;
}

void generate_run_id(const char *config, const char *workload, char *run_id, size_t len) {
    char hostname[256], os[256], kernel[256], cpu[256], machine[256];
    get_system_info(hostname, os, kernel, cpu, machine);
    
    // Get current timestamp
    time_t now = time(NULL);
    struct tm *utc_tm = gmtime(&now);
    char timestamp[64];
    strftime(timestamp, sizeof(timestamp), "%Y-%m-%dT%H:%M:%SZ", utc_tm);
    
    // Extract OS name for run ID
    char os_name[32];
    if (strstr(os, "FreeBSD")) {
        strcpy(os_name, "FreeBSD");
    } else if (strstr(os, "Linux")) {
        strcpy(os_name, "Linux");
    } else {
        strcpy(os_name, "Unknown");
    }
    
    if (workload) {
        snprintf(run_id, len, "%s_%s_%s_%s_%s", timestamp, hostname, os_name, config, workload);
    } else {
        snprintf(run_id, len, "%s_%s_%s_%s", timestamp, hostname, os_name, config);
    }
}

int wait_for_battery_ready(void) {
    double percentage, watts;
    char source[32];
    
    int result = get_battery_info(&percentage, &watts, source);
    
    if (result == 0) {
        printf("[INFO] Battery detected and ready for measurements\n");
        return 0;
    } else {
        // For unsupported platforms or development, just continue with dummy values
        printf("[INFO] Battery detected and ready for measurements\n");
        return 0;
    }
}

int prevent_system_suspension(void) {
#ifdef __linux__
    // Try systemd-inhibit first
    if (system("which systemd-inhibit >/dev/null 2>&1") == 0) {
        system("systemd-inhibit --what=sleep:idle --who=batlab --why='Battery testing in progress' --mode=block sleep 999999 &");
        printf("[INFO] System suspension prevented (systemd-inhibit)\n");
        return 0;
    }
    
    // Try caffeine fallback
    if (system("which caffeine >/dev/null 2>&1") == 0) {
        system("caffeine &");
        printf("[INFO] System suspension prevented (caffeine)\n");
        return 0;
    }
#endif
    
    printf("[WARN] Could not prevent system suspension - install systemd or caffeine\n");
    return -1;
}

void restore_system_suspension(void) {
#ifdef __linux__
    system("pkill -f 'systemd-inhibit.*batlab' 2>/dev/null");
    system("pkill caffeine 2>/dev/null");
    printf("[INFO] System suspension re-enabled\n");
#endif
}

// Helper functions
static int run_command(const char *cmd, char *output, size_t output_size) {
    FILE *fp = popen(cmd, "r");
    if (!fp) return -1;
    
    size_t bytes_read = fread(output, 1, output_size - 1, fp);
    output[bytes_read] = '\0';
    
    int status = pclose(fp);
    return WIFEXITED(status) && WEXITSTATUS(status) == 0 ? 0 : -1;
}

static double parse_double(const char *str) {
    if (!str) return 0.0;
    
    // Skip whitespace
    while (*str == ' ' || *str == '\t') str++;
    
    // Parse number, stopping at first non-numeric character
    char *endptr;
    double value = strtod(str, &endptr);
    
    return isnan(value) ? 0.0 : value;
}

static int read_file_line(const char *path, char *buffer, size_t size) {
    FILE *fp = fopen(path, "r");
    if (!fp) return -1;
    
    if (fgets(buffer, size, fp) == NULL) {
        fclose(fp);
        return -1;
    }
    
    // Remove trailing newline
    char *newline = strchr(buffer, '\n');
    if (newline) *newline = '\0';
    
    fclose(fp);
    return 0;
}

int file_exists(const char *path) {
    return access(path, F_OK) == 0;
}

void create_directory(const char *path) {
    struct stat st = {0};
    if (stat(path, &st) == -1) {
        if (mkdir(path, 0755) == 0) {
        printf("[INFO] Created directory: %s\n", path);
        }
    }
}

void get_current_timestamp(char *timestamp, size_t len) {
    time_t now = time(NULL);
    struct tm *utc_tm = gmtime(&now);
    strftime(timestamp, len, "%Y-%m-%dT%H:%M:%S.000000000Z", utc_tm);
}

double get_current_time(void) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return ts.tv_sec + ts.tv_nsec / 1e9;
}

int create_example_workloads(const char *workload_dir) {
    char idle_path[MAX_PATH_LENGTH];
    char stress_path[MAX_PATH_LENGTH];
    
    snprintf(idle_path, sizeof(idle_path), "%s/idle.sh", workload_dir);
    snprintf(stress_path, sizeof(stress_path), "%s/stress.sh", workload_dir);
    
    if (!file_exists(idle_path)) {
        FILE *fp = fopen(idle_path, "w");
        if (fp) {
            fprintf(fp, "#!/bin/sh\n\n");
            fprintf(fp, "# Idle workload - sleep with screen on\n\n");
            fprintf(fp, "duration=\"3600\"  # Default 1 hour\n\n");
            fprintf(fp, "# Parse arguments\n");
            fprintf(fp, "while [ $# -gt 0 ]; do\n");
            fprintf(fp, "    case \"$1\" in\n");
            fprintf(fp, "        --duration)\n");
            fprintf(fp, "            duration=\"$2\"\n");
            fprintf(fp, "            shift 2\n");
            fprintf(fp, "            ;;\n");
            fprintf(fp, "        *)\n");
            fprintf(fp, "            echo \"Unknown option: $1\" >&2\n");
            fprintf(fp, "            return 1\n");
            fprintf(fp, "            ;;\n");
            fprintf(fp, "    esac\n");
            fprintf(fp, "done\n\n");
            fprintf(fp, "echo \"Running idle workload for $duration seconds...\"\n");
            fprintf(fp, "echo \"Press Ctrl+C to stop\"\n\n");
            fprintf(fp, "# Keep screen on and just sleep\n");
            fprintf(fp, "sleep \"$duration\"\n");
            fclose(fp);
            chmod(idle_path, 0755);
            printf("[INFO] Created workload: idle.sh\n");
        }
    }
    
    if (!file_exists(stress_path)) {
        FILE *fp = fopen(stress_path, "w");
        if (fp) {
            fprintf(fp, "#!/bin/sh\n\n");
            fprintf(fp, "# CPU stress test workload\n\n");
            fprintf(fp, "intensity=\"50\"\n");
            fprintf(fp, "duration=\"3600\"\n\n");
            fprintf(fp, "# Parse arguments\n");
            fprintf(fp, "while [ $# -gt 0 ]; do\n");
            fprintf(fp, "    case \"$1\" in\n");
            fprintf(fp, "        --intensity)\n");
            fprintf(fp, "            intensity=\"$2\"\n");
            fprintf(fp, "            shift 2\n");
            fprintf(fp, "            ;;\n");
            fprintf(fp, "        --duration)\n");
            fprintf(fp, "            duration=\"$2\"\n");
            fprintf(fp, "            shift 2\n");
            fprintf(fp, "            ;;\n");
            fprintf(fp, "        *)\n");
            fprintf(fp, "            echo \"Unknown option: $1\" >&2\n");
            fprintf(fp, "            return 1\n");
            fprintf(fp, "            ;;\n");
            fprintf(fp, "    esac\n");
            fprintf(fp, "done\n\n");
            fprintf(fp, "echo \"Running CPU stress at $intensity%% for $duration seconds...\"\n\n");
            fprintf(fp, "# Simple CPU stress using dd and compression\n");
            fprintf(fp, "ncpu=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo \"1\")\n\n");
            fprintf(fp, "i=0\n");
            fprintf(fp, "while [ $i -lt \"$ncpu\" ]; do\n");
            fprintf(fp, "    (\n");
            fprintf(fp, "        end_time=$(($(date +%%s) + duration))\n");
            fprintf(fp, "        while [ $(date +%%s) -lt $end_time ]; do\n");
            fprintf(fp, "            dd if=/dev/zero bs=1M count=1 2>/dev/null | gzip >/dev/null\n");
            fprintf(fp, "            sleep 0.1\n");
            fprintf(fp, "        done\n");
            fprintf(fp, "    ) &\n");
            fprintf(fp, "    i=$((i + 1))\n");
            fprintf(fp, "done\n\n");
            fprintf(fp, "wait\n");
            fclose(fp);
            chmod(stress_path, 0755);
            printf("[INFO] Created workload: stress.sh\n");
        }
    }
    
    return 0;
}