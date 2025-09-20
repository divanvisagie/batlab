# HTML Templates

This directory contains HTML templates and CSS files for the batlab reporting system.

**Note:** As of the C implementation, emoji have been removed from all workload scripts for maximum system compatibility.

## Template Structure

### Main Templates
- `index.html.template` - Dashboard page template with placeholder variables
- `report.html.template` - Individual battery report template

### CSS Stylesheets
- `index-styles.css` - Styling for the main dashboard
- `report-styles.css` - Styling for individual reports

### Partial Templates
- `no-reports.html.partial` - HTML fragment for when no reports exist
- `report-card.html.partial` - HTML fragment for report cards in the grid

## How Templates Work

Templates use placeholder variables in the format `{{VARIABLE_NAME}}` that get replaced with actual values by the `batlab-report` script.

### Index Template Variables
- `{{LOGO_SRC}}` - Path to batlab logo
- `{{TOTAL_REPORTS}}` - Number of generated reports
- `{{UNIQUE_HOSTS}}` - Number of unique devices tested
- `{{CURRENT_YEAR}}` - Current year
- `{{REPORTS_SECTION}}` - Dynamic content (report grid or no-reports message)
- `{{LAST_UPDATED}}` - Generation timestamp

### Report Template Variables
- `{{CONFIG_NAME}}` - Test configuration name
- `{{HOST}}` - Host system name
- `{{OS}}` - Operating system
- `{{START_TIME}}` - Test start timestamp
- `{{RUN_ID}}` - Unique run identifier
- `{{SAMPLING_HZ}}` - Data sampling rate
- `{{DURATION}}` - Test duration in hours
- `{{SAMPLES}}` - Number of data samples
- `{{START_PCT}}` - Starting battery percentage
- `{{END_PCT}}` - Ending battery percentage
- `{{BATTERY_DRAIN}}` - Total battery drain percentage
- `{{DRAIN_RATE}}` - Battery drain rate per hour
- `{{AVG_WATTS}}` - Average power consumption
- `{{MIN_WATTS}}` - Minimum power consumption
- `{{MAX_WATTS}}` - Maximum power consumption
- `{{AVG_CPU}}` - Average CPU utilization
- `{{AVG_TEMP}}` - Average temperature
- `{{REPORT_NAME}}` - Report filename (for graph reference)
- `{{GENERATION_DATE}}` - Report generation date
- `{{DATA_SOURCE}}` - Source data filename

## Editing Templates

### To modify styling:
1. Edit the CSS files directly (`index-styles.css`, `report-styles.css`)
2. Changes apply immediately to new reports
3. Existing reports need to be regenerated to pick up changes

### To modify HTML structure:
1. Edit the template files
2. Keep placeholder variables intact: `{{VARIABLE_NAME}}`
3. The `batlab-report` script currently uses heredoc generation, not the template files
4. To use templates, modify the script's `generate_html_report()` and `generate_index()` functions

## Design Theme

The templates use a FreeBSD-inspired design:
- **Colors**: FreeBSD red (`#cc0000`) with variants (`#990000`, `#660000`, `#aa3333`)
- **Background**: Clean white (`#ffffff`)
- **Fonts**: Monospaced fonts for technical readability
- **Layout**: Professional, research-quality presentation

## File Organization

```
templates/
├── README.md                 # This file
├── index.html.template       # Main dashboard template
├── report.html.template      # Individual report template
├── index-styles.css          # Dashboard CSS
├── report-styles.css         # Report CSS
├── no-reports.html.partial   # No reports message
└── report-card.html.partial  # Report card component
```

## System Compatibility

All emoji have been removed from workload scripts to ensure compatibility across different systems:
- FreeBSD base system terminals
- Linux console environments
- SSH connections without Unicode support
- Older terminal emulators

## Future Improvements

The current system has templates available but uses heredoc generation in the bash script for simplicity. To fully utilize the template system:

1. Implement the `render_template()` function properly
2. Handle multiline content and special characters safely
3. Add more template variables for customization
4. Create additional partial templates for reusable components

## Usage with batlab-report

Templates are referenced by the `batlab-report` script:
- CSS files are linked relatively in generated HTML
- Template files provide the structure and styling foundation
- The script generates HTML using the template patterns