import os
import re

dir_path = "/Users/claywarren/PDAL/rust/pdal-filters/src"
for filename in os.listdir(dir_path):
    if filename == "lib.rs" or filename == "merge.rs" or not filename.endswith(".rs"):
        continue
    
    path = os.path.join(dir_path, filename)
    with open(path, 'r') as f:
        content = f.read()
    
    # Replace the signature and inject the input extraction
    # We match the exact signature and use a replacement that adds the new line
    new_content = re.sub(
        r'fn run\(&mut self, input: &PointView\) -> Result<Vec<PointView>, StageError> \{',
        r'fn run(&mut self, inputs: &[PointView]) -> Result<Vec<PointView>, StageError> {\n        let input = inputs.first().ok_or_else(|| StageError("Missing input".into()))?;',
        content
    )
    
    if new_content != content:
        with open(path, 'w') as f:
            f.write(new_content)
        print(f"Updated {filename}")
    else:
        print(f"Skipped {filename} (no match)")
