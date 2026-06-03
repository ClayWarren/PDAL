//! Stage registry -- construct implemented stages from PDAL driver names.
//!
//! This is the name-keyed slice of PDAL's `StageFactory`, restricted to the
//! reader/filter/writer drivers this Rust spike currently implements.

use pdal_core::options::Options;
use pdal_core::pipeline::{Reader, Writer};
use pdal_core::stage::StageError;

mod filters;
mod pipeline;

pub use filters::create_filter;
pub(crate) use pipeline::options_from_object;
pub use pipeline::{pdal_pipeline_create_json, pipeline_from_json};

pub const READER_DRIVERS: &[&str] = &[
    "readers.faux",
    "readers.bpf",
    "readers.fbi",
    "readers.gdal",
    "readers.text",
    "readers.pcd",
    "readers.pts",
    "readers.ptx",
    "readers.ilvis2",
    "readers.obj",
    "readers.optech",
    "readers.qfit",
    "readers.sbet",
    "readers.smrmsg",
    "readers.terrasolid",
    "readers.copc",
    "readers.ept",
    "readers.las",
    "readers.laz",
    "readers.nitf",
    "readers.ply",
    "readers.spz",
    "readers.stac",
    "readers.tindex",
];

pub const FILTER_DRIVERS: &[&str] = &[
    "filters.approximatecoplanar",
    "filters.assign",
    "filters.chipper",
    "filters.cluster",
    "filters.colorinterp",
    "filters.colorization",
    "filters.covariancefeatures",
    "filters.crop",
    "filters.csf",
    "filters.dbscan",
    "filters.decimation",
    "filters.delaunay",
    "filters.dem",
    "filters.divider",
    "filters.eigenvalues",
    "filters.elm",
    "filters.estimaterank",
    "filters.expression",
    "filters.expressionstats",
    "filters.ferry",
    "filters.faceraster",
    "filters.geomdistance",
    "filters.gpstimeconvert",
    "filters.groupby",
    "filters.hag_delaunay",
    "filters.hag_dem",
    "filters.hag_nn",
    "filters.h3",
    "filters.head",
    "filters.hexbin",
    "filters.iqr",
    "filters.label_duplicates",
    "filters.litree",
    "filters.lloydkmeans",
    "filters.m3c2",
    "filters.locate",
    "filters.lof",
    "filters.mad",
    "filters.merge",
    "filters.miniball",
    "filters.mongo",
    "filters.mortonorder",
    "filters.neighborclassifier",
    "filters.nndistance",
    "filters.normal",
    "filters.optimalneighborhood",
    "filters.outlier",
    "filters.overlay",
    "filters.planefit",
    "filters.pmf",
    "filters.projpipeline",
    "filters.radialdensity",
    "filters.radiusassign",
    "filters.randomize",
    "filters.range",
    "filters.reciprocity",
    "filters.relaxationdartthrowing",
    "filters.reprojection",
    "filters.returns",
    "filters.smrf",
    "filters.farthestpointsampling",
    "filters.sample",
    "filters.separatescanline",
    "filters.skewnessbalancing",
    "filters.sparsesurface",
    "filters.splitter",
    "filters.sort",
    "filters.stats",
    "filters.straighten",
    "filters.supervoxel",
    "filters.tail",
    "filters.transformation",
    "filters.voxelcenternearestneighbor",
    "filters.voxelcentroidnearestneighbor",
    "filters.voxeldownsize",
    "filters.zsmooth",
];

pub const WRITER_DRIVERS: &[&str] = &[
    "writers.null",
    "writers.bpf",
    "writers.copc",
    "writers.fbi",
    "writers.gltf",
    "writers.text",
    "writers.pcd",
    "writers.sbet",
    "writers.las",
    "writers.laz",
    "writers.nitf",
    "writers.ply",
    "writers.ogr",
    "writers.gdal",
    "writers.raster",
    "writers.spz",
];

pub enum CreatedStage {
    Reader(Box<dyn Reader>),
    Filter(Box<dyn pdal_core::pipeline::StageWrapper>),
    Writer(Box<dyn Writer>),
}

pub fn create_reader(name: &str, options: &Options) -> Result<Box<dyn Reader>, StageError> {
    match name {
        "readers.faux" => pdal_io::faux::FauxReader::new(options)
            .map(|reader| Box::new(reader) as Box<dyn Reader>)
            .map_err(StageError),
        "readers.bpf" => Ok(Box::new(pdal_io::bpf::BpfReader::new(options))),
        "readers.fbi" => Ok(Box::new(pdal_io::fbi::FbiReader::new(options))),
        "readers.gdal" => Ok(Box::new(pdal_io::gdal_reader::GdalReader::new(options))),
        "readers.text" => Ok(Box::new(pdal_io::text::TextReader::new(options))),
        "readers.pcd" => Ok(Box::new(pdal_io::pcd::PcdReader::new(options))),
        "readers.pts" => Ok(Box::new(pdal_io::pts::PtsReader::new(options))),
        "readers.ptx" => Ok(Box::new(pdal_io::ptx::PtxReader::new(options))),
        "readers.ilvis2" => Ok(Box::new(pdal_io::ilvis2::Ilvis2Reader::new(options))),
        "readers.obj" => Ok(Box::new(pdal_io::obj::ObjReader::new(options))),
        "readers.optech" => Ok(Box::new(pdal_io::optech::OptechReader::new(options))),
        "readers.qfit" => Ok(Box::new(pdal_io::qfit::QfitReader::new(options))),
        "readers.sbet" => Ok(Box::new(pdal_io::sbet::SbetReader::new(options))),
        "readers.smrmsg" => Ok(Box::new(pdal_io::smrmsg::SmrmsgReader::new(options))),
        "readers.terrasolid" => Ok(Box::new(pdal_io::terrasolid::TerrasolidReader::new(
            options,
        ))),
        "readers.copc" => Ok(Box::new(pdal_io::copc::CopcReader::new(options))),
        "readers.las" | "readers.laz" => Ok(Box::new(pdal_io::las::LasReader::new(options))),
        "readers.nitf" => Ok(Box::new(pdal_io::nitf_reader::NitfReader::new(options))),
        "readers.ept" => Ok(Box::new(pdal_io::ept::EptReader::new(options))),
        "readers.ply" => Ok(Box::new(pdal_io::ply::PlyReader::new(options))),
        "readers.spz" => Ok(Box::new(pdal_io::spz::SpzReader::new(options))),
        "readers.stac" => Ok(Box::new(pdal_io::stac::StacReader::new(options))),
        "readers.tindex" => Ok(Box::new(pdal_io::tindex::TindexReader::new(options))),
        _ => Err(StageError(format!(
            "Reader driver '{name}' is not available in the Rust port."
        ))),
    }
}

pub(crate) fn get_f64(options: &Options, key: &str, default: f64) -> Result<f64, StageError> {
    options.try_get_f64(key, default).map_err(StageError)
}

pub(crate) fn get_u64(options: &Options, key: &str, default: u64) -> Result<u64, StageError> {
    options.try_get_u64(key, default).map_err(StageError)
}

pub(crate) fn get_bool(options: &Options, key: &str, default: bool) -> Result<bool, StageError> {
    options.try_get_bool(key, default).map_err(StageError)
}

pub fn create_writer(name: &str, options: &Options) -> Result<Box<dyn Writer>, StageError> {
    match name {
        "writers.null" => Ok(Box::new(pdal_io::nullwriter::NullWriter::new(options))),
        "writers.bpf" => Ok(Box::new(pdal_io::bpf::BpfWriter::new(options))),
        "writers.fbi" => Ok(Box::new(pdal_io::fbi_writer::FbiWriter::new(options))),
        "writers.gltf" => Ok(Box::new(pdal_io::gltf::GltfWriter::new(options))),
        "writers.text" => Ok(Box::new(pdal_io::text_writer::TextWriter::new(options))),
        "writers.pcd" => Ok(Box::new(pdal_io::pcd::PcdWriter::new(options))),
        "writers.sbet" => Ok(Box::new(pdal_io::sbet_writer::SbetWriter::new(options))),
        "writers.las" => Ok(Box::new(pdal_io::las_writer::LasWriter::new(options))),
        "writers.laz" => Ok(Box::new(pdal_io::las_writer::LasWriter::new_laz(options))),
        "writers.copc" => {
            // COPC requires LAS 1.4; default minor_version to 4 like the C ABI.
            let mut opts = options.clone();
            if !opts.has("minor_version") {
                opts.add("minor_version", "4");
            }
            Ok(Box::new(pdal_io::copcwriter::writer::CopcWriter::new(
                &opts,
            )))
        }
        "writers.nitf" => Ok(Box::new(pdal_io::nitf_writer::NitfWriter::new(options)?)),
        "writers.ply" => Ok(Box::new(pdal_io::ply::PlyWriter::new(options)?)),
        "writers.ogr" => Ok(Box::new(pdal_io::ogr_writer::OgrWriter::new(options))),
        "writers.gdal" => Ok(Box::new(pdal_io::gdal_writer::GdalWriter::new(options))),
        "writers.raster" => Ok(Box::new(pdal_io::raster_writer::RasterWriter::new(options))),
        "writers.spz" => Ok(Box::new(pdal_io::spz::SpzWriter::new(options))),
        _ => Err(StageError(format!(
            "Writer driver '{name}' is not available in the Rust port."
        ))),
    }
}

pub fn create_stage(name: &str, options: &Options) -> Result<CreatedStage, StageError> {
    if name.starts_with("readers.") {
        create_reader(name, options).map(CreatedStage::Reader)
    } else if name.starts_with("writers.") {
        create_writer(name, options).map(CreatedStage::Writer)
    } else if name.starts_with("filters.") {
        create_filter(name, options).map(CreatedStage::Filter)
    } else {
        Err(StageError(format!(
            "Stage driver '{name}' is not available in the Rust port."
        )))
    }
}

#[cfg(test)]
mod pipeline_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod value_tests;
