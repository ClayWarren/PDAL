import os
import re

filters_dir = "/Users/claywarren/PDAL/rust/pdal-filters/src/"
special_files = ["decimation.rs", "head.rs", "tail.rs", "locate.rs", "expression.rs", "expressionstats.rs", "mongo.rs"]

def update_file(filepath):
    filename = os.path.basename(filepath)
    with open(filepath, 'r') as f:
        content = f.read()

    if filename == "assign.rs":
        # Specific logic for assign.rs
        old_pattern = r"fn process_one\(\s*&mut self,\s*_view: &pdal_core::point::PointView,\s*_idx: pdal_core::point::PointId,\s*\) -> bool \{\s*false\s*\}"
        new_logic = """fn process_one(&mut self, view: &mut PointView, idx: PointId) -> bool {
        self.assign_point(view, idx);
        true
    }"""
        if re.search(old_pattern, content):
            content = re.sub(old_pattern, new_logic, content)
        else:
             # Try another pattern if the first one fails
             old_pattern2 = r"fn process_one\(\s*&mut self,\s*_view: &PointView,\s*_idx: PointId,\s*\) -> bool \{\s*false\s*\}"
             content = re.sub(old_pattern2, new_logic, content)
    
    elif filename == "h3.rs":
        # Specific logic for h3.rs
        old_signature = r"fn process_one\(&mut self, view: &PointView, idx: PointId\) -> bool \{"
        new_signature = "fn process_one(&mut self, view: &mut PointView, idx: PointId) -> bool {"
        content = content.replace(old_signature, new_signature)
        
        # Implement logic
        logic_replacement = """                let cell = latlng.to_cell(res);
                view.set_f64(idx, &DimId::H3, u64::from(cell) as f64);"""
        content = re.sub(r"// Note: view\.set_f64 is not possible if view is &PointView\..*?// Let's check the signature in rust/pdal-core/src/stage\.rs\.", logic_replacement, content, flags=re.DOTALL)

    elif filename in special_files:
        # Just change signature, keep logic
        content = content.replace("_view: &pdal_core::point::PointView", "view: &mut pdal_core::point::PointView")
        content = content.replace("_view: &PointView", "view: &mut PointView")
        content = content.replace("view: &pdal_core::point::PointView", "view: &mut pdal_core::point::PointView")
        content = content.replace("view: &PointView", "view: &mut PointView")
    
    else:
        # Stub: change signature and ensure parameters are ignored
        # Pattern for stub implementations
        def replace_stub(match):
            indent = match.group(1)
            # Use &mut PointView and ignore parameters
            return f"{indent}fn process_one(\n{indent}    &mut self,\n{indent}    _view: &mut PointView,\n{indent}    _idx: PointId,\n{indent}) -> bool {{\n{indent}    false\n{indent}}}"

        # Match various forms of stubs
        content = re.sub(r"(\s*)fn process_one\(\s*&mut self,\s*(_?view): &(?:pdal_core::point::)?PointView,\s*(_?idx): (?:pdal_core::point::)?PointId,?\s*\) -> bool \{\s*false\s*\}", replace_stub, content)

    with open(filepath, 'w') as f:
        f.write(content)

for filename in os.listdir(filters_dir):
    if filename.endswith(".rs") and filename != "lib.rs":
        update_file(os.path.join(filters_dir, filename))

# Update pdal-capi/src/lib.rs
capi_file = "/Users/claywarren/PDAL/rust/pdal-capi/src/lib.rs"
with open(capi_file, 'r') as f:
    capi_content = f.read()

capi_content = capi_content.replace("fn process_one(&mut self, view: &PointView, idx: u64) -> bool;", "fn process_one(&mut self, view: &mut PointView, idx: u64) -> bool;")
capi_content = capi_content.replace("fn process_one(&mut self, view: &PointView, idx: u64) -> bool {", "fn process_one(&mut self, view: &mut PointView, idx: u64) -> bool {")

with open(capi_file, 'w') as f:
    f.write(capi_content)
