#!/usr/bin/env python3
import re
import os

sdk_path = os.environ.get('SDK')
headers_dir = f"{sdk_path}/System/Library/Frameworks/Network.framework/Headers"

symbols = {}

for header_file in sorted(os.listdir(headers_dir)):
    if not header_file.endswith('.h'):
        continue
    
    header_path = os.path.join(headers_dir, header_file)
    with open(header_path, 'r') as f:
        content = f.read()
    
    # Skip if file contains API_UNAVAILABLE for macOS
    lines = content.split('\n')
    for i, line in enumerate(lines):
        # Look for function declarations starting with nw_ or sec_
        if re.match(r'^nw_|^sec_', line) and '(' in line:
            # Extract function name
            match = re.search(r'(\bnw_\w+|sec_\w+)\s*\(', line)
            if match:
                func_name = match.group(1)
                # Check if this line or nearby lines have API_UNAVAILABLE
                context = '\n'.join(lines[max(0,i-5):min(len(lines),i+1)])
                if 'API_UNAVAILABLE' in context and 'macos' in context.lower():
                    continue
                symbols[func_name] = {
                    'kind': 'function',
                    'header': header_file,
                    'context': line[:100]
                }

print(f"Found {len(symbols)} public C function symbols")
for sym in sorted(symbols.keys())[:20]:
    print(f"  {sym}")
print(f"  ...")
