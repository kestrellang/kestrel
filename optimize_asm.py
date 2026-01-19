import re
import sys

def optimize_asm(input_path, output_path):
    with open(input_path, 'r') as f:
        lines = f.readlines()

    functions = {}
    current_func = None
    current_lines = []
    
    # Pass 1: Extract functions
    for line in lines:
        # Check for function start (label ending in colon, not indented)
        match = re.match(r'^([^ \t\n]+):$', line)
        if match:
            if current_func:
                functions[current_func] = current_lines
            current_func = match.group(1)
            current_lines = [line]
        elif current_func:
            current_lines.append(line)
    
    if current_func:
        functions[current_func] = current_lines

    # Pass 2: Build call graph
    reachable = set()
    to_visit = ["_main"]
    
    while to_visit:
        func_name = to_visit.pop()
        if func_name in reachable:
            continue
        reachable.add(func_name)
        
        if func_name in functions:
            for line in functions[func_name]:
                # Look for 'bl <target>' or 'b <target>'
                # otool output format: 000000010002f704	bl	__K2io5stdio5printP1R3std4text6String
                call_match = re.search(r'\bbl\b\s+([^\s]+)', line)
                if call_match:
                    target = call_match.group(1).strip()
                    # Remove surrounding quotes if any
                    target = target.strip('"')
                    if target in functions:
                        to_visit.append(target)
                
                # Also check for tail calls 'b <target>'
                # But be careful not to match local labels or conditional jumps
                jump_match = re.search(r'^[0-9a-f]+\s+b\s+([^\s]+)', line)
                if jump_match:
                    target = jump_match.group(1).strip()
                    target = target.strip('"')
                    if target in functions and not target.startswith('0x'):
                         to_visit.append(target)

    # Pass 3: Optimize reachable functions
    optimized_functions = {}
    for func_name in reachable:
        if func_name not in functions:
            continue
        
        func_lines = functions[func_name]
        new_lines = []
        i = 0
        while i < len(func_lines):
            # Check for 8-byte copy pattern
            # ldrb w0, [xN]
            # strb w0, [xM]
            # ldrb w1, [xN, #0x1]
            # strb w1, [xM, #0x1]
            # ...
            pattern_matched = False
            if i + 15 < len(func_lines):
                potential_copy = func_lines[i:i+16]
                regs = []
                base_src = None
                base_dst = None
                valid = True
                for j in range(8):
                    ld = potential_copy[j*2]
                    st = potential_copy[j*2+1]
                    
                    # Match ldrb w{j}, [x{src}(, #0x{j})?]
                    ld_m = re.search(r'ldrb\s+w(\d+),\s+\[([^,\]]+)(?:,\s+#0x([0-9a-f]+))?\]', ld)
                    st_m = re.search(r'strb\s+w(\d+),\s+\[([^,\]]+)(?:,\s+#0x([0-9a-f]+))?\]', st)
                    
                    if not ld_m or not st_m:
                        valid = False
                        break
                    
                    reg_ld, src, off_ld = ld_m.groups()
                    reg_st, dst, off_st = st_m.groups()
                    
                    if reg_ld != reg_st or int(reg_ld) != j:
                        valid = False
                        break
                    
                    if base_src is None: base_src = src
                    elif base_src != src: 
                        valid = False
                        break
                        
                    if base_dst is None: base_dst = dst
                    elif base_dst != dst:
                        valid = False
                        break
                    
                    expected_off = f'{j:x}' if j > 0 else None
                    if off_ld != expected_off or off_st != expected_off:
                        # Some versions might show #0x0 or nothing
                        if j == 0 and (off_ld in [None, '0'] and off_st in [None, '0']):
                            pass
                        else:
                            valid = False
                            break
                
                if valid:
                    # Replace with ldr x0, [base_src]; str x0, [base_dst]
                    addr = potential_copy[0].split('\t')[0]
                    new_lines.append(f"{addr}\tldr\tx0, [{base_src}]\n")
                    addr2 = potential_copy[1].split('\t')[0]
                    new_lines.append(f"{addr2}\tstr\tx0, [{base_dst}]\t; OPTIMIZED: 8-byte copy\n")
                    i += 16
                    pattern_matched = True
            
            if not pattern_matched:
                new_lines.append(func_lines[i])
                i += 1    
        
        optimized_functions[func_name] = new_lines

    # Output result
    with open(output_path, 'w') as f:
        f.write("./array2:\n(__TEXT,__text) section\n")
        # Sort functions to keep output somewhat consistent, main first
        sorted_funcs = sorted(optimized_functions.keys(), key=lambda x: (x != "_main", x))
        for func in sorted_funcs:
            f.writelines(optimized_functions[func])
            f.write("\n")

if __name__ == "__main__":
    optimize_asm("array2.asm", "array2.asm.optimized")
