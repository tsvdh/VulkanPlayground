import sys
import uuid
import pyautogui
import time

# shader path is shaders\$dir_name$\$shader_name$
# rust module declaration is assumed to be in $root$\src\bin\$dir_name$\shader_modules.rs

root = sys.argv[1]
shader_path = sys.argv[2]
shader_path = shader_path.replace("\\", "/")

rust_file_path = "".join([root, "\\src\\bin\\", shader_path.split("/")[1], "\\shader_modules.rs"])

### update edit id in shader_modules.rs ###

with open(rust_file_path) as rust_file:
    rust_file_contents = rust_file.readlines()

uuid_line = None
for i, line in enumerate(rust_file_contents):
    if shader_path in line:
        uuid_line = i + 1

if uuid_line is None:
    raise Exception("Cannot find line to replace")

valid_uuid = str(uuid.uuid4())
valid_uuid = valid_uuid.replace("0", "x")
valid_uuid = valid_uuid.replace("f", "x")

uuid_line_start = rust_file_contents[uuid_line].split("[")[0]
uuid_line_end = '[("edit_id", "__X__")]\n'.replace("__X__", valid_uuid)

rust_file_contents[uuid_line] = "".join([uuid_line_start, uuid_line_end])

with open(rust_file_path, "w") as rust_file:
    rust_file.writelines(rust_file_contents)

######

### open and close shader_modules.rs ###

time.sleep(0.5)

pyautogui.hotkey("ctrl", "shift", "f")
pyautogui.write(shader_path)
time.sleep(0.1)
pyautogui.press("enter")
pyautogui.hotkey("ctrl", "f4")
