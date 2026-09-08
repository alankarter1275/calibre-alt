import re

with open('src/pages/comics_reader/mod.rs', 'r') as f:
    content = f.read()

# We need to replace:
# set_child: Some(&gtk::Box { ... }),
# with:
# #[wrap(Some)]
# set_child = &gtk::Box { ... },

def repl(m):
    return '#[wrap(Some)]\n                                        set_child = &gtk::Box {'

content = re.sub(r'set_child:\s*Some\(&gtk::Box\s*\{', repl, content)

with open('src/pages/comics_reader/mod.rs', 'w') as f:
    f.write(content)

