import re

with open('src/pages/comics_reader/mod.rs', 'r') as f:
    content = f.read()

# Replace `}),` with `},` if preceded by the gtk::Label that I inserted.
# Actually, I can just replace `}),` with `},` inside the specific sections, or generally for those specific buttons.

content = re.sub(r'add_css_class: "kalam-comics-seg-label" \},\n\s*\w*\)\}', r'add_css_class: "kalam-comics-seg-label" },\n                                        }', content)

content = re.sub(r'set_icon_size: gtk::IconSize::Inherit \},\n\s*\w*\)\}', r'set_icon_size: gtk::IconSize::Inherit },\n                                        }', content)

with open('src/pages/comics_reader/mod.rs', 'w') as f:
    f.write(content)

