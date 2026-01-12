
import subprocess
import glob
from collections import defaultdict
import os

def run_check():
    # Find all .ks files in lang/std/
    files = glob.glob('lang/std/**/*.ks', recursive=True)
    if not files:
        print("No .ks files found in lang/std/")
        return ""

    command = ["cargo", "run", "--", "check"] + files
    print(f"Running: {' '.join(command)}")
    
    # Run the command and capture output, while printing to console
    process = subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1
    )

    full_output = []
    for line in process.stdout:
        print(line, end="")
        full_output.append(line)
    
    process.wait()
    return "".join(full_output)

def parse_errors(output):
    errors = defaultdict(list)
    current_error = None
    
    lines = output.splitlines()
    for line in lines:
        line_stripped = line.strip()
        if line_stripped.startswith('error: '):
            current_error = line_stripped[len('error: '):]
        elif current_error and line_stripped.startswith('┌─ '):
            call_site = line_stripped[3:]
            errors[current_error].append(call_site)
            current_error = None
                
    return errors

def write_report(errors, output_path):
    # Sort by count descending, then by name
    sorted_errors = sorted(errors.items(), key=lambda x: (-len(x[1]), x[0]))
    
    with open(output_path, 'w') as f:
        f.write("1. Summary\n\n")
        f.write("| Error Message | Count |\n")
        f.write("| :--- | :--- |\n")
        for msg, sites in sorted_errors:
            f.write(f"| {msg} | {len(sites)} |\n")
        
        f.write("\n2. Details\n")
        for msg, sites in sorted_errors:
            f.write(f"\n{msg}\n")
            f.write("Call Sites\n")
            for site in sites:
                f.write(f"- {site}\n")

if __name__ == "__main__":
    output = run_check()
    if output:
        errors = parse_errors(output)
        write_report(errors, "errors.md")
        print(f"\nReport written to errors.md")
