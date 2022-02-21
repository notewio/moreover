import json
from pprint import pprint
import re
import argparse
import json
import re


NUMBERS = {
    '0': 'O',
    '1': 'S',
    '2': 'T',
    '3': 'P',
    '4': 'H',
    '5': 'A',
    '6': 'F',
    '7': 'P',
    '8': 'L',
    '9': 'T',
}

def normalize(stroke):
    has_num = False
    for num, correct in NUMBERS.items():
        if num in stroke:
            has_num = True
            stroke = stroke.replace(num, correct)
    if has_num:
        stroke = "#" + stroke

    split = ["", "", ""]
    right = False
    for char in stroke:
        if char in "AOEU*-":
            right = True
            split[1] += char
        else:
            if right:
                split[2] += char
            else:
                split[0] += char

    split[2] = split[2].lower()
    split[1] = split[1].replace("E", "e")
    split[1] = split[1].replace("U", "u")
    split[1] = split[1].replace("-", "")

    return ''.join(split)

PSEUDO = [
    ("STKPW", "Z"),
    ("TKPW", "G"),
    ("SKWR", "J"),
    ("TPH", "N"),
    ("KWR", "Y"),
    ("SR", "V"),
    ("TK", "D"),
    ("PW", "B"),
    ("HR", "L"),
    ("TP", "F"),
    ("PH", "M"),
    
    # Ae, Au, Ai, Oe, Ou, Oi all make sense already
    ("AOeu", "ii"),
    ("AOe", "ee"),
    ("AOu", "uu"),
    ("AO", "oo"),
    ("eu", "i"),
    
    ("frpb", "nch"),
    ("pblg", "j"),
    ("frp", "mp"),
    ("frb", "rv"),
    ("pb", "n"),
    ("pl", "m"),
    ("bg", "k"),
    ("gs", "tion"),
    ("fp", "ch"),
    ("rb", "sh"),
]

def to_pseudo(stroke):
    used = set()
    for s, p in PSEUDO:
        if any([x in used for x in s]):
            continue
        new = stroke.replace(s, p)
        if new != stroke:
            stroke = new
            for x in s:
                used.add(x)
    return stroke

parser = argparse.ArgumentParser(description="Convert Plover JSON dictionaries to custom format.")
parser.add_argument("input", help="Input file")
parser.add_argument("output", help="Output file")
args = parser.parse_args()

with open(args.input, "r", encoding="utf-8") as f:
    content = json.load(f)

final = {}

print("Converting stroke format...")
for key, value in content.items():
    new = []
    key = key.split("/")

    for stroke in key:
        stroke = normalize(stroke)
        stroke = to_pseudo(stroke)
        new.append(stroke)

    new = ' '.join(new)
    
    if "\n" in value:
        value = value.replace("\n", "\\n")

    carry_cap = re.search("\{(\^)?~\|(.+?)(\^)?\}", value)
    if carry_cap:
        new_value = ""
        attach_before = carry_cap.group(1)
        if attach_before: new_value += "{^}"
        text = carry_cap.group(2)
        new_value += "{~|}" + text
        attach_after = carry_cap.group(3)
        if attach_after: new_value += "{^}"
        value = re.sub("\{(\^)?~\|(.+?)(\^)?\}", new_value, value)

    final[new] = value


print("Generating tree...")
tree = ["", {}]
for k, v in sorted(final.items(), key=lambda x: len(x[0].split())):
    strokes = k.split()
    if len(strokes) == 1:
        tree[1][k] = [
            v,
            {}
        ]
    else:
        parent = tree
        i = 0
        while i < len(strokes) and strokes[i] in parent[1]:
            parent = parent[1][strokes[i]]
            i += 1
        while i < len(strokes):
            parent[1][strokes[i]] = [
                "",
                {}
            ]
            parent = parent[1][strokes[i]]
            i += 1
        parent[0] = v

def render(t):
    output = ""
    for k, v in t[1].items():
        output += render_helper(k, v)
    return output

def render_helper(key, children):
    output = ""
    if children[0]:
        output = '{}\t{}\n'.format(key, children[0])
    else:
        output = '{}\n'.format(key)
    for k, v in children[1].items():
        output += "\n".join(["\t" + x for x in render_helper(k, v).strip().split("\n")]) + "\n"
    return output

with open(args.output, "w", encoding="utf-8") as f:
    f.write(render(tree))