#!/usr/bin/env python3
import urllib.request
import re
import sys
import os
import subprocess
import argparse
import json

# Split the regions to prevent backend 500 errors
GROUP1 = "eu-central-1,eu-west-1,eu-west-2,us-east-1,us-east-2,us-west-1,us-west-2,ca-central-1,sa-east-1"
GROUP2 = "ap-south-1,ap-east-1,ap-northeast-1,ap-northeast-2,ap-southeast-1,ap-southeast-2"

# Region AWS mappings
API_TO_AWS = {
    "Frankfurt": "eu-central-1", "Dublin": "eu-west-1", "London": "eu-west-2",
    "Virginia": "us-east-1", "Ohio": "us-east-2", "California": "us-west-1",
    "Oregon": "us-west-2", "Montréal": "ca-central-1", "São Paulo": "sa-east-1",
    "Mumbai": "ap-south-1", "Hong Kong": "ap-east-1", "Tokyo": "ap-northeast-1",
    "Seoul": "ap-northeast-2", "Singapore": "ap-southeast-1", "Sydney": "ap-southeast-2"
}
ALL_AWS_REGIONS = list(API_TO_AWS.values())

CONFIG_DIR = os.path.expanduser("~/.config/dbdqueue")
CONFIG_FILE = os.path.join(CONFIG_DIR, "config.json")

# ANSI Color codes
C_RESET = "\033[0m"
C_BOLD = "\033[1m"
C_GREEN = "\033[92m"
C_YELLOW = "\033[93m"
C_RED = "\033[91m"
C_CYAN = "\033[96m"
C_GRAY = "\033[90m"
C_REVERSE = "\033[7m"

ANSI_ESCAPE = re.compile(r'\x1b(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')

def get_terminal_width(s):
    """Calculate the actual display column width of a string in a terminal,
    correctly counting double-width emojis/flags and ignoring invisible ANSI escape codes."""
    s_clean = ANSI_ESCAPE.sub('', s)
    width = 0
    for char in s_clean:
        code = ord(char)
        if 0x1F1E6 <= code <= 0x1F1FF:
            width += 1
        elif 0x1F300 <= code <= 0x1F9FF or 0x2600 <= code <= 0x27BF:
            width += 2
        else:
            width += 1
    return width

def pad_right(s, target_width):
    """Pads string with spaces to reach target_width in terminal display columns."""
    current_w = get_terminal_width(s)
    if current_w >= target_width:
        return s
    return s + " " * (target_width - current_w)

def load_config():
    os.makedirs(CONFIG_DIR, exist_ok=True)
    default = {"priority": [], "locked": [], "sort": "default", "mode": "both"}
    if os.path.exists(CONFIG_FILE):
        try:
            with open(CONFIG_FILE, "r") as f:
                return {**default, **json.load(f)}
        except Exception:
            pass
    return default

def save_config(config):
    try:
        with open(CONFIG_FILE, "w") as f:
            json.dump(config, f, indent=2)
    except Exception:
        pass

def parse_time_to_seconds(time_str):
    if time_str == "—" or not time_str:
        return 999999
    match = re.match(r"(?:(\d+)m)?(?:(\d+)s)?", time_str)
    if not match:
        return 999999
    m, s = match.groups()
    total = 0
    if m: total += int(m) * 60
    if s: total += int(s)
    return total

def colorize_time(time_str):
    if time_str == "—":
        return C_GRAY + "—" + C_RESET
    sec = parse_time_to_seconds(time_str)
    if sec < 60:
        return C_GREEN + time_str + C_RESET
    elif sec < 180:
        return C_YELLOW + time_str + C_RESET
    else:
        return C_RED + time_str + C_RESET

def clean_region_name(raw_name):
    cleaned = re.sub(r'[^\w\s\u00C0-\u017F]+', '', raw_name)
    return cleaned.replace("Event", "").strip()

def fetch_queue_times():
    responses = []
    headers = {
        'User-Agent': 'curl/8.7.1',
        'Accept': '*/*'
    }
    for group in [GROUP1, GROUP2]:
        url = f"https://api.deadbyqueue.com/queuetime?region={group}&mode=live,live-event&extras=flag,regionname"
        try:
            req = urllib.request.Request(url, headers=headers)
            with urllib.request.urlopen(req, timeout=10) as response:
                responses.append(response.read().decode('utf-8'))
        except Exception as e:
            print(f"{C_RED}Error fetching data:{C_RESET} {e}", file=sys.stderr)
            sys.exit(1)

    combined_text = " | ".join(responses)
    data = []
    
    for p in [x.strip() for x in combined_text.split("|") if x.strip()]:
        m = re.match(
            r"^(.*?)\s+([^\s]+)\s*/\s*([^\s,]+)(?:,\s*Event:\s*([^\s]+)\s*/\s*([^\s]+))?$",
            p
        )
        if m:
            flag_and_name, k_std, s_std, k_ev, s_ev = m.groups()
            emoji_match = re.match(r"^([^\w\s]+)\s*(.*)$", flag_and_name)
            if emoji_match:
                flag, name = emoji_match.groups()
            else:
                flag, name = "", flag_and_name
            
            name_clean = clean_region_name(name)
            
            data.append({
                "flag": flag,
                "name": name_clean,
                "mode": "Standard",
                "survivor": s_std,
                "killer": k_std
            })
            
            if k_ev and s_ev:
                data.append({
                    "flag": flag,
                    "name": name_clean,
                    "mode": "Event",
                    "survivor": s_ev,
                    "killer": k_ev
                })
        else:
            emoji_match = re.match(r"^([^\w\s]+)\s*(.*)$", p)
            if emoji_match:
                flag, name = emoji_match.groups()
            else:
                flag, name = "", p
            
            name = name.replace("❌", "").replace("Offline", "").replace(",", "").replace("Event:", "").strip()
            name_clean = clean_region_name(name)
            
            data.append({
                "flag": flag,
                "name": name_clean,
                "mode": "Standard",
                "survivor": "—",
                "killer": "—"
            })
            data.append({
                "flag": flag,
                "name": name_clean,
                "mode": "Event",
                "survivor": "—",
                "killer": "—"
            })
            
    return data

def build_hosts_content(selected_aws_regions):
    START = "# --- DBD REGION CHANGER START ---"
    END   = "# --- DBD REGION CHANGER END ---"
    hosts_path = "/etc/hosts"

    try:
        with open(hosts_path, "r", encoding="utf-8", errors="ignore") as f:
            lines = f.readlines()
    except Exception as e:
        print(f"{C_RED}Error reading hosts file:{C_RESET} {e}", file=sys.stderr)
        return None

    new_lines, in_block = [], False
    for line in lines:
        if line.strip() == START:
            in_block = True; continue
        if line.strip() == END:
            in_block = False; continue
        if not in_block:
            new_lines.append(line)

    while new_lines and new_lines[-1].strip() == "":
        new_lines.pop()
    new_lines.append("\n")

    if selected_aws_regions is not None and len(selected_aws_regions) < len(API_TO_AWS):
        new_lines.append(START + "\n# Do not edit manually. Generated by dbdqueue CLI\n")
        for reg in ALL_AWS_REGIONS:
            if reg not in selected_aws_regions:
                for ep in [
                    f"gamelift-ping.{reg}.api.aws",
                    f"gamelift.{reg}.api.aws",
                    f"gamelift-ping.{reg}.amazonaws.com",
                    f"gamelift.{reg}.amazonaws.com",
                ]:
                    new_lines.append(f"0.0.0.0 {ep}\n::1 {ep}\n")
        new_lines.append(END + "\n")

    return "".join(new_lines)

def update_hosts(selected_aws_regions):
    new_content = build_hosts_content(selected_aws_regions)
    if new_content is None:
        return

    try:
        with open("/etc/hosts", "r", encoding="utf-8", errors="ignore") as f:
            current_content = f.read()
    except Exception:
        current_content = ""

    if current_content == new_content:
        print(f"{C_GREEN}Hosts file is already up to date.{C_RESET}")
        return

    print(f"{C_YELLOW}Requesting write access to /etc/hosts...{C_RESET}")
    try:
        proc = subprocess.Popen(
            ["pkexec", "tee", "/etc/hosts"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True
        )
        stdout, stderr = proc.communicate(input=new_content)
        if proc.returncode != 0:
            print(f"{C_RED}Failed to write to /etc/hosts (code {proc.returncode}):{C_RESET}\n{stderr.strip()}", file=sys.stderr)
        else:
            print(f"{C_GREEN}Successfully updated region locks!{C_RESET}")
            import shutil
            if shutil.which("resolvectl"):
                subprocess.run(["resolvectl", "flush-caches"], capture_output=True)
            elif shutil.which("systemd-resolve"):
                subprocess.run(["systemd-resolve", "--flush-caches"], capture_output=True)
    except Exception as e:
        print(f"{C_RED}Error updating hosts:{C_RESET} {e}", file=sys.stderr)

# ==================== INTERACTIVE KEY READER ====================
def getch():
    import tty
    import termios
    fd = sys.stdin.fileno()
    old_settings = termios.tcgetattr(fd)
    try:
        tty.setraw(sys.stdin.fileno())
        ch = sys.stdin.read(1)
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old_settings)
    return ch

def get_key():
    ch = getch()
    if ch == '\x1b':
        ch2 = getch()
        if ch2 == '[':
            ch3 = getch()
            if ch3 == 'A': return 'up'
            elif ch3 == 'B': return 'down'
            elif ch3 == 'C': return 'right'
            elif ch3 == 'D': return 'left'
    elif ch == '\r' or ch == '\n':
        return 'enter'
    elif ch == ' ':
        return 'space'
    elif ch == '\x03':  # Ctrl+C
        return 'ctrl+c'
    return ch

def interactive_lock_menu(current_locked):
    # Prepare list of options
    regions = sorted(list(API_TO_AWS.keys()))
    selected = {API_TO_AWS[r]: (API_TO_AWS[r] in current_locked) for r in regions}
    cursor_pos = 0

    # Hide cursor
    sys.stdout.write("\033[?25l")
    sys.stdout.flush()

    try:
        while True:
            # Clear terminal lines
            # Move cursor to top of the menu and clear
            sys.stdout.write("\033[H\033[2J")  # Full clear screen
            print(f"{C_BOLD}{C_CYAN}=== DBD Region Locker (Interactive Mode) ==={C_RESET}")
            print("Navigate: ARROWS | Toggle: SPACE | Lock & Save: ENTER | Quit: Ctrl+C\n")

            for i, r in enumerate(regions):
                aws_code = API_TO_AWS[r]
                checked = "[*]" if selected[aws_code] else "[ ]"
                color = C_GREEN if selected[aws_code] else C_GRAY
                
                if i == cursor_pos:
                    print(f" {C_REVERSE} {checked} {r} ({aws_code}) {C_RESET}")
                else:
                    print(f" {color} {checked} {r} ({aws_code}){C_RESET}")

            key = get_key()
            if key == 'up':
                cursor_pos = (cursor_pos - 1) % len(regions)
            elif key == 'down':
                cursor_pos = (cursor_pos + 1) % len(regions)
            elif key == 'space':
                aws_code = API_TO_AWS[regions[cursor_pos]]
                selected[aws_code] = not selected[aws_code]
            elif key == 'enter':
                break
            elif key == 'ctrl+c':
                print(f"\n{C_YELLOW}Cancelled.{C_RESET}")
                sys.stdout.write("\033[?25h")
                sys.stdout.flush()
                sys.exit(0)
    finally:
        # Show cursor
        sys.stdout.write("\033[?25h")
        sys.stdout.flush()

    locked_list = [aws for aws, sel in selected.items() if sel]
    return locked_list

def interactive_priority_menu(current_priority):
    # Prepare list of options
    regions = sorted(list(API_TO_AWS.keys()))
    selected = {r: (r in current_priority) for r in regions}
    cursor_pos = 0

    # Hide cursor
    sys.stdout.write("\033[?25l")
    sys.stdout.flush()

    try:
        while True:
            # Clear terminal lines
            # Move cursor to top of the menu and clear
            sys.stdout.write("\033[H\033[2J")  # Full clear screen
            print(f"{C_BOLD}{C_CYAN}=== DBD Priority Regions (Interactive Mode) ==={C_RESET}")
            print("Navigate: ARROWS | Toggle: SPACE | Save: ENTER | Quit: Ctrl+C\n")

            for i, r in enumerate(regions):
                checked = "[*]" if selected[r] else "[ ]"
                color = C_GREEN if selected[r] else C_GRAY
                
                if i == cursor_pos:
                    print(f" {C_REVERSE} {checked} {r} {C_RESET}")
                else:
                    print(f" {color} {checked} {r}{C_RESET}")

            key = get_key()
            if key == 'up':
                cursor_pos = (cursor_pos - 1) % len(regions)
            elif key == 'down':
                cursor_pos = (cursor_pos + 1) % len(regions)
            elif key == 'space':
                r = regions[cursor_pos]
                selected[r] = not selected[r]
            elif key == 'enter':
                break
            elif key == 'ctrl+c':
                print(f"\n{C_YELLOW}Cancelled.{C_RESET}")
                sys.stdout.write("\033[?25h")
                sys.stdout.flush()
                sys.exit(0)
    finally:
        # Show cursor
        sys.stdout.write("\033[?25h")
        sys.stdout.flush()

    priority_list = [r for r in regions if selected[r]]
    return priority_list

# ==================== TABLE DRAWING HELPER ====================
def draw_table_rows(rows, col_width_region, col_width_mode, col_width_surv, col_width_kill, priority_list):
    border_mid = f"├{'─' * (col_width_region+2)}┼{'─' * (col_width_mode+2)}┼{'─' * (col_width_surv+2)}┼{'─' * (col_width_kill+2)}┤"
    for i, row in enumerate(rows):
        flag_spaced = f"{row['flag']} " if row['flag'] else ""
        reg_text = f"{flag_spaced}{row['name']}"
        
        # Color priority names in table
        if row['name'] in priority_list:
            reg_text = f"{C_CYAN}{reg_text}{C_RESET}"
            
        r_cell = pad_right(reg_text, col_width_region)
        m_cell = pad_right(row['mode'], col_width_mode)
        
        s_val = colorize_time(row['survivor'])
        k_val = colorize_time(row['killer'])
        
        # Format ANSI padded string cells
        s_colored = s_val + " " * (col_width_surv - get_terminal_width(row['survivor']))
        k_colored = k_val + " " * (col_width_kill - get_terminal_width(row['killer']))
        
        print(f"│ {r_cell} │ {m_cell} │ {s_colored} │ {k_colored} │")
        if i < len(rows) - 1:
            print(border_mid)

AWS_TO_API = {v: k for k, v in API_TO_AWS.items()}

def parse_priority_input(words_list):
    # Join into a single string first to handle commas and spaces
    raw_str = " ".join(words_list)
    
    # Split by comma first
    parts = []
    for p in raw_str.split(","):
        p_clean = p.strip()
        if p_clean:
            parts.append(p_clean)
            
    normalized_map = {
        "sao paulo": "São Paulo",
        "sao_paulo": "São Paulo",
        "saopaulo": "São Paulo",
        "hong kong": "Hong Kong",
        "hong_kong": "Hong Kong",
        "hongkong": "Hong Kong",
        "montreal": "Montréal",
    }
    
    resolved = []
    for part in parts:
        part_lower = part.lower()
        if part_lower in normalized_map:
            resolved.append(normalized_map[part_lower])
            continue
        if part_lower in AWS_TO_API:
            resolved.append(AWS_TO_API[part_lower])
            continue
            
        part_cap = part.capitalize()
        if part_cap in API_TO_AWS:
            resolved.append(part_cap)
            continue
            
        # If the part contains multiple words (like "Frankfurt Dublin Sao Paulo")
        words = part.split()
        i = 0
        while i < len(words):
            word = words[i].lower()
            if i + 1 < len(words):
                two_words = f"{word} {words[i+1].lower()}"
                if two_words in normalized_map:
                    resolved.append(normalized_map[two_words])
                    i += 2
                    continue
            
            if word in normalized_map:
                resolved.append(normalized_map[word])
            elif word in AWS_TO_API:
                resolved.append(AWS_TO_API[word])
            else:
                word_cap = word.capitalize()
                if word_cap in API_TO_AWS:
                    resolved.append(word_cap)
            i += 1
            
    return resolved

def main():
    parser = argparse.ArgumentParser(
        description="Dead by Daylight Queue Times & Region Locker CLI",
        formatter_class=argparse.RawDescriptionHelpFormatter
    )
    
    subparsers = parser.add_subparsers(dest="command")
    
    # lock subcommand
    lock_parser = subparsers.add_parser("lock", help="Lock regions (blocking all others)")
    lock_parser.add_argument("regions", nargs="*", help="Regions to whitelist (leave empty for interactive menu)")
    
    # unlock subcommand
    subparsers.add_parser("unlock", help="Unlock all regions")
    
    # show options (default command)
    parser.add_argument("-s", "--sort", choices=["survivor", "killer", "priority", "default"],
                        help="Sort output by column/rules (persists in config)")
    parser.add_argument("-m", "--mode", choices=["standard", "event", "both"],
                        help="Filter rows by Mode (persists in config)")
    parser.add_argument("-p", "--priority", nargs="*", help="Set priority regions in config (comma or space separated, leave empty for interactive menu)")
    
    args = parser.parse_args()
    config = load_config()

    # Apply configuration updates from flags
    config_changed = False
    if args.sort:
        config["sort"] = args.sort
        config_changed = True
        print(f"{C_GREEN}Default sorting set to:{C_RESET} {args.sort}")

    if args.priority is not None:
        if args.priority == []:
            priorities = interactive_priority_menu(config.get("priority", []))
        else:
            priorities = parse_priority_input(args.priority)
        config["priority"] = priorities
        config_changed = True
        print(f"{C_GREEN}Updated priority list to:{C_RESET} {', '.join(priorities)}")

    if config_changed:
        save_config(config)

    # Subcommand: Lock
    if args.command == "lock":
        if not args.regions:
            # Open interactive menu
            resolved_regions = interactive_lock_menu(config.get("locked", []))
        else:
            resolved_regions = []
            priority_resolved = parse_priority_input(args.regions)
            for r in priority_resolved:
                resolved_regions.append(API_TO_AWS[r])
        config["locked"] = resolved_regions
        save_config(config)
        update_hosts(resolved_regions)
        sys.exit(0)

    # Subcommand: Unlock
    elif args.command == "unlock":
        config["locked"] = []
        save_config(config)
        update_hosts(None)
        sys.exit(0)

    # Fetch data and filter
    data = fetch_queue_times()
    
    # Filter by Mode (args.mode overrides config, but is not saved)
    active_mode = args.mode if args.mode else config.get("mode", "both")
    if active_mode == "standard":
        data = [r for r in data if r["mode"] == "Standard"]
    elif active_mode == "event":
        data = [r for r in data if r["mode"] == "Event"]

    # Filter priority and standard lists
    priority_names = config.get("priority", [])
    
    priority_rows = [r for r in data if r["name"] in priority_names]
    other_rows = [r for r in data if r["name"] not in priority_names]

    # Apply Sorting
    active_sort = config.get("sort", "default")
    
    def get_sort_key(sort_type):
        if sort_type == "survivor":
            return lambda x: parse_time_to_seconds(x["survivor"])
        elif sort_type == "killer":
            return lambda x: parse_time_to_seconds(x["killer"])
        else:
            return lambda x: x["name"]

    if active_sort in ["survivor", "killer"]:
        priority_rows.sort(key=get_sort_key(active_sort))
        other_rows.sort(key=get_sort_key(active_sort))
    elif active_sort == "priority":
        # Keep priority order as specified in the config list
        priority_rows.sort(key=lambda x: priority_names.index(x["name"]))
        other_rows.sort(key=lambda x: x["name"])

    # Draw Table Layout
    col_width_region = 22
    col_width_mode = 10
    col_width_surv = 10
    col_width_kill = 10
    
    border_top = f"┌{'─' * (col_width_region+2)}┬{'─' * (col_width_mode+2)}┬{'─' * (col_width_surv+2)}┬{'─' * (col_width_kill+2)}┐"
    border_mid = f"├{'─' * (col_width_region+2)}┼{'─' * (col_width_mode+2)}┼{'─' * (col_width_surv+2)}┼{'─' * (col_width_kill+2)}┤"
    border_double = f"╞{'═' * (col_width_region+2)}╪{'═' * (col_width_mode+2)}╪{'═' * (col_width_surv+2)}╪{'═' * (col_width_kill+2)}╡"
    border_bot = f"└{'─' * (col_width_region+2)}┴{'─' * (col_width_mode+2)}┴{'─' * (col_width_surv+2)}┴{'─' * (col_width_kill+2)}┘"
    
    print(border_top)
    r_hdr = pad_right("Region", col_width_region)
    m_hdr = pad_right("Mode", col_width_mode)
    s_hdr = pad_right("Survivor", col_width_surv)
    k_hdr = pad_right("Killer", col_width_kill)
    print(f"│ {C_BOLD}{r_hdr}{C_RESET} │ {C_BOLD}{m_hdr}{C_RESET} │ {C_BOLD}{s_hdr}{C_RESET} │ {C_BOLD}{k_hdr}{C_RESET} │")
    print(border_double)
    
    # Draw all rows (priority rows placed at the top)
    all_rows = priority_rows + other_rows
    if all_rows:
        draw_table_rows(all_rows, col_width_region, col_width_mode, col_width_surv, col_width_kill, priority_names)
        
    # Print final bottom border line
    print(border_bot)

if __name__ == "__main__":
    main()
