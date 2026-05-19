    fn run_tindex(&self) -> i32 {
        if self.help || self.command_args.len() < 3 {
            println!("Usage:");
            println!("  pdal tindex create <tindex> <files...>");
            println!("  pdal tindex merge <tindex> <filespec>");
            return if self.command_args.is_empty() && !self.help {
                1
            } else {
                0
            };
        }

        let subcommand = &self.command_args[0];
        if subcommand == "merge" {
            eprintln!("Error: merge is not yet supported in the Rust tindex kernel");
            return 1;
        } else if subcommand != "create" {
            eprintln!("Error: expected 'create' or 'merge' subcommand");
            return 1;
        }

        let tindex_file = &self.command_args[1];
        let files = &self.command_args[2..];

        // Register drivers
        pdal_core::gdal::register_drivers();

        // Create OGR dataset
        let dataset = match pdal_core::gdal::Vector::create(tindex_file, "ESRI Shapefile") {
            Ok(ds) => ds,
            Err(e) => {
                eprintln!("Error creating tindex dataset: {}", e);
                return 1;
            }
        };

        // For first file, we get its SRS to define the layer
        let mut first_srs = String::new();
        let mut first_bounds = None;
        let mut valid_files = Vec::new();
        
        for file in files {
            let driver = match pdal_core::driver::infer_reader_driver(file) {
                Some(driver) => driver,
                None => continue,
            };
            
            let pipeline_json = serde_json::json!([{ "type": driver, "filename": file }]).to_string();
            let c_json = match CString::new(pipeline_json) {
                Ok(json) => json,
                Err(_) => continue,
            };
            
            let pipeline = unsafe { pdal_capi::pdal_pipeline_create_json(c_json.as_ptr()) };
            if pipeline.is_null() { continue; }
            
            let json_ptr = unsafe { pdal_capi::pdal_pipeline_execute_summary_json(pipeline, std::ptr::null_mut()) };
            unsafe { pdal_capi::pdal_pipeline_destroy(pipeline) };
            
            if json_ptr.is_null() { continue; }
            
            let summary_str = safe_cstr(json_ptr).unwrap_or_default();
            unsafe { pdal_capi::pdal_string_free(json_ptr) };
            
            if let Ok(summary) = serde_json::from_str::<serde_json::Value>(&summary_str) {
                let wkt = summary["metadata"]["pipeline"]["stage_0"]["srs"]["wkt"].as_str().unwrap_or("").to_string();
                let minx = summary["bounds_2d"]["minx"].as_f64().unwrap_or(0.0);
                let maxx = summary["bounds_2d"]["maxx"].as_f64().unwrap_or(0.0);
                let miny = summary["bounds_2d"]["miny"].as_f64().unwrap_or(0.0);
                let maxy = summary["bounds_2d"]["maxy"].as_f64().unwrap_or(0.0);
                
                if first_srs.is_empty() && !wkt.is_empty() {
                    first_srs = wkt.clone();
                }
                valid_files.push((file.clone(), wkt, minx, miny, maxx, maxy));
            }
        }
        
        if valid_files.is_empty() {
            eprintln!("Error: no valid files to index");
            return 1;
        }

        let layer = match dataset.open_or_create_layer("pdal", &first_srs) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error creating layer: {}", e);
                return 1;
            }
        };

        pdal_core::gdal::Vector::create_string_field(layer, "location").unwrap();
        pdal_core::gdal::Vector::create_string_field(layer, "srs").unwrap();
        pdal_core::gdal::Vector::create_datetime_field(layer, "created").unwrap();
        pdal_core::gdal::Vector::create_datetime_field(layer, "modified").unwrap();

        for (file, wkt, minx, miny, maxx, maxy) in valid_files {
            // WKT for POLYGON
            let poly_wkt = format!("POLYGON (({} {}, {} {}, {} {}, {} {}, {} {}))",
                minx, miny,
                maxx, miny,
                maxx, maxy,
                minx, maxy,
                minx, miny
            );
            
            let fields = vec![
                ("location", file.as_str()),
                ("srs", wkt.as_str()),
            ];
            
            if let Err(e) = pdal_core::gdal::Vector::add_feature(layer, &poly_wkt, &fields) {
                eprintln!("Error adding feature for {}: {}", file, e);
            } else {
                println!("Indexed file {}", file);
            }
        }

        0
    }