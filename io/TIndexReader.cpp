/******************************************************************************
 * Copyright (c) 2015, Howard Butler (howard@hobu.co)
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following
 * conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright
 *       notice, this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright
 *       notice, this list of conditions and the following disclaimer in
 *       the documentation and/or other materials provided
 *       with the distribution.
 *     * Neither the name of Hobu, Inc. or Flaxen Geo Consulting nor the
 *       names of its contributors may be used to endorse or promote
 *       products derived from this software without specific prior
 *       written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS
 * OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
 * AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT
 * OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 ****************************************************************************/

#include "TIndexReader.hpp"

#include <ogr_api.h>

#include <pdal/Polygon.hpp>
#include <pdal/StageWrapper.hpp>
#include <pdal/private/OGRSpec.hpp>
#include <pdal/private/gdal/GDALUtils.hpp>
#include <pdal/private/gdal/SpatialRef.hpp>
#include <pdal/util/ProgramArgs.hpp>
#include <pdal/util/private/JsonSupport.hpp>

#include <nlohmann/json.hpp>

#include <sstream>

namespace pdal
{

static StaticPluginInfo const s_info{
    "readers.tindex",
    "TileIndex Reader",
    "https://pdal.org/stages/readers.tindex.html",
    {"tindex"}};

CREATE_STATIC_STAGE(TIndexReader, s_info)

namespace
{

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
    pdal_options_add_str(options, key.c_str(), value.c_str());
}

void throwLastRustError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

bool hasSuffix(const std::string& value, const std::string& suffix)
{
    return value.size() >= suffix.size() &&
           value.compare(value.size() - suffix.size(), suffix.size(), suffix) ==
               0;
}

} // namespace

std::string TIndexReader::getName() const
{
    return s_info.name;
}

struct TIndexReader::Args
{
    std::string m_layerName;
    std::string m_driverName;
    std::string m_tileIndexColumnName;
    std::string m_srsColumnName;
    std::string m_wkt;
    OGRSpec m_ogr;
    std::string m_tgtSrsString;
    std::string m_filterSRS;
    std::string m_attributeFilter;
    std::string m_dialect;
    BOX2D m_bounds;
    std::string m_sql;
    std::vector<NL::json> m_rawReaderArgs;
    NL::json m_readerArgs;
};

TIndexReader::TIndexReader()
    : m_args(new TIndexReader::Args), m_dataset(nullptr), m_layer(nullptr)
{
}

TIndexReader::~TIndexReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

TIndexReader::FieldIndexes TIndexReader::getFields()
{
    FieldIndexes indexes;

    OGRFeatureDefnH fDefn = OGR_L_GetLayerDefn(m_layer);

    indexes.m_filename =
        OGR_FD_GetFieldIndex(fDefn, m_args->m_tileIndexColumnName.c_str());
    if (indexes.m_filename < 0)
        throwError("Unable to find field '" + m_args->m_tileIndexColumnName +
                   "' in file '" + m_filename + "'.");
    if (m_args->m_srsColumnName.size())
        indexes.m_srs =
            OGR_FD_GetFieldIndex(fDefn, m_args->m_srsColumnName.c_str());

    indexes.m_ctime = OGR_FD_GetFieldIndex(fDefn, "created");
    indexes.m_mtime = OGR_FD_GetFieldIndex(fDefn, "modified");

    return indexes;
}

std::vector<TIndexReader::FileInfo> TIndexReader::getFiles()
{
    std::vector<TIndexReader::FileInfo> output;

    OGR_L_ResetReading(m_layer);
    FieldIndexes indexes = getFields();

    while (true)
    {
        OGRFeatureH feature = OGR_L_GetNextFeature(m_layer);
        if (!feature)
            break;

        FileInfo fileInfo;
        fileInfo.m_filename =
            OGR_F_GetFieldAsString(feature, indexes.m_filename);

        if (m_args->m_srsColumnName.size())
        {
            fileInfo.m_srs = OGR_F_GetFieldAsString(feature, indexes.m_srs);
        }
        output.push_back(fileInfo);

        OGR_F_Destroy(feature);
    }

    return output;
}

void TIndexReader::addArgs(ProgramArgs& args)
{
    args.add("lyr_name", "OGR layer name from which to read tile index layer",
             m_args->m_layerName, "pdal");
    args.addSynonym("lyr_name", "layer");

    args.add("srs_column", "Column to use to override a file's SRS",
             m_args->m_srsColumnName, "");
    args.add("tindex_name",
             "OGR column name from which to read tile "
             "index location",
             m_args->m_tileIndexColumnName, "location");
    args.add("sql",
             "OGR-compatible SQL statement for querying tile "
             "index layer",
             m_args->m_sql);
    args.add("bounds",
             "Bounds box to limit query window. "
             "Format: '([xmin,xmax],[ymin,ymax])'",
             m_args->m_bounds);
    args.add("polygon", "Well-known text description of bounds to limit query",
             m_args->m_wkt);
    args.addSynonym("polygon", "wkt");
    args.add("ogr", "Specified OGR polygon to limit query", m_args->m_ogr);
    args.add("t_srs", "Transform SRS of tile index geometry",
             m_args->m_tgtSrsString, "EPSG:4326");
    args.add("filter_srs",
             "Transforms any wkt or boundary option to "
             "this coordinate system before filtering or reading data.",
             m_args->m_filterSRS, "EPSG:4326");
    args.add("where",
             "OGR SQL filter clause to use on the layer. It only "
             "works in combination with tile index layers that are defined "
             "with lyr_name",
             m_args->m_attributeFilter);
    args.add("dialect",
             "OGR SQL dialect to use when querying tile "
             "index layer",
             m_args->m_dialect, "OGRSQL");
    args.add("reader_args",
             "Map of reader arguments to their values to pass through.",
             m_args->m_rawReaderArgs);
}

void TIndexReader::addDimensions(PointLayoutPtr layout)
{
    if (m_useRustReader)
    {
        m_rustDims.clear();
        m_rustDimNames.clear();
        uint64_t dimCount = pdal_point_view_dim_count(m_rustView);
        for (uint64_t idx = 0; idx < dimCount; ++idx)
        {
            char* rawName = pdal_point_view_dim_name(m_rustView, idx);
            if (!rawName)
                continue;
            std::string name(rawName);
            pdal_string_free(rawName);
            Dimension::Id id =
                layout->registerOrAssignDim(name, Dimension::Type::Double);
            m_rustDims.push_back(id);
            m_rustDimNames.push_back(name);
        }
        return;
    }

    layout->registerDim(pdal::Dimension::Id::X);
    layout->registerDim(pdal::Dimension::Id::Y);
    layout->registerDim(pdal::Dimension::Id::Z);
}

void TIndexReader::initialize()
{
    m_useRustReader = canUseRustReader();
    if (m_useRustReader)
    {
        loadRustView();
        return;
    }

    if (!m_args->m_bounds.empty())
        m_args->m_wkt = m_args->m_bounds.toWKT();
    m_out_ref.reset(new gdal::SpatialRef());

    log()->get(LogLevel::Debug) << "Opening file " << m_filename << '\n';

    gdal::registerDrivers();
    m_dataset = OGROpen(m_filename.c_str(), FALSE, nullptr);
    if (!m_dataset)
        throwError("Unable to datasource '" + m_filename + "'");

    OGRGeometryH geometry(nullptr);
    if (m_args->m_sql.size())
    {
        m_layer = OGR_DS_ExecuteSQL(m_dataset, m_args->m_sql.c_str(), geometry,
                                    m_args->m_dialect.c_str());
    }
    else
    {
        m_layer = OGR_DS_GetLayerByName(m_dataset, m_args->m_layerName.c_str());
    }
    if (!m_layer)
        throwError("Unable to open layer '" + m_args->m_layerName +
                   "' from OGR datasource '" + m_filename + "'");

    m_out_ref->setFromLayer(m_layer);

    // Override the SRS if the user set one, otherwise, take it
    // from the layer
    if (m_args->m_tgtSrsString.size())
        m_out_ref.reset(new gdal::SpatialRef(m_args->m_tgtSrsString));
    else
        m_out_ref.reset(new gdal::SpatialRef(m_out_ref->wkt()));

    // Set SRS if not overridden.
    if (getSpatialReference().empty())
        setSpatialReference(SpatialReference(m_out_ref->wkt()));

    // If an OGR specification was added, we overwrite the wkt polygon
    // with it. If OGRSpec is going to contain non-polygon geometries
    // in the future this method would need to be changed.
    if (m_args->m_ogr.size())
    {
        Polygon ogrPoly = m_args->m_ogr.getPolygons()[0];
        m_args->m_wkt = ogrPoly.wkt();
    }
    // If the user set either explicit 'polygon' or 'boundary' options
    // we will filter by that geometry. The user can set a 'filter_srs'
    // option to override the SRS of the input geometry and we will
    // reproject to the output projection as needed.
    Polygon poly;
    if (m_args->m_wkt.size())
    {
        // Reproject the given wkt to the output SRS so
        // filtering/cropping works
        poly = Polygon(m_args->m_wkt, m_args->m_filterSRS);
        poly.transform(m_out_ref->wkt());

        m_args->m_wkt = poly.wkt();
        OGR_L_SetSpatialFilter(m_layer, poly.getOGRHandle());
    }

    if (m_args->m_attributeFilter.size())
    {
        OGRErr err = OGR_L_SetAttributeFilter(
            m_layer, m_args->m_attributeFilter.c_str());
        if (err != OGRERR_NONE)
            throwError("Unable to set attribute filter '" +
                       m_args->m_attributeFilter + "' for OGR datasource '" +
                       m_filename + "'");
    }

    Options cropOptions;
    if (m_args->m_wkt.size())
        cropOptions.add("polygon", m_args->m_wkt);

    if (m_args->m_rawReaderArgs.size())
        m_args->m_readerArgs = Utils::handleReaderArgs(m_args->m_rawReaderArgs);

    for (const auto& f : getFiles())
    {
        log()->get(LogLevel::Debug)
            << "Adding file " << f.m_filename << " to merge filter" << '\n';

        std::string driver = m_factory.inferReaderDriver(f.m_filename);
        Stage* reader = m_factory.createStage(driver);
        if (!reader)
            throwError("Unable to create reader for file '" + f.m_filename +
                       "'.");
        reader->setLog(log());

        Options readerOptions =
            Utils::setReaderOptions(m_args->m_readerArgs, driver, f.m_filename);

        reader->setOptions(readerOptions);
        Stage* premerge = reader;

        if (m_args->m_tgtSrsString.size())
        {
            Stage* repro = m_factory.createStage("filters.reprojection");
            repro->setInput(*reader);
            Options reproOptions;
            reproOptions.add("out_srs", m_args->m_tgtSrsString);
            if (m_args->m_srsColumnName.size())
            {
                reproOptions.add("in_srs", f.m_srs);
                log()->get(LogLevel::Debug2)
                    << "Repro = " << m_args->m_tgtSrsString << "/" << f.m_srs
                    << "!\n";
            }
            repro->setOptions(reproOptions);
            premerge = repro;
        }

        // WKT is set even if we're using a bounding box for filtering, so
        // can be used as a test here.
        if (!m_args->m_wkt.empty())
        {
            Stage* crop = m_factory.createStage("filters.crop");
            crop->setOptions(cropOptions);
            crop->setInput(*premerge);
            log()->get(LogLevel::Debug3)
                << "Cropping data with wkt '" << m_args->m_wkt.substr(0, 400)
                << "......'" << '\n';
            premerge = crop;
        }

        m_merge.setInput(*premerge);
    }

    if (m_args->m_sql.size())
    {
        // We were created with OGR_DS_ExecuteSQL which needs to have
        // its layer explicitly released
        OGR_DS_ReleaseResultSet(m_dataset, m_layer);
    }
    else
    {
        OGR_DS_Destroy(m_dataset);
    }
    m_layer = nullptr;
    m_dataset = nullptr;

    setInput(m_merge);
}

point_count_t TIndexReader::read(PointViewPtr view, point_count_t num)
{
    if (m_useRustReader)
    {
        point_count_t count = 0;
        while (count < num && m_rustIndex < pdal_point_view_length(m_rustView))
        {
            PointRef point(view->point(view->size()));
            copyRustPoint(point, m_rustIndex);
            ++m_rustIndex;
            ++count;
        }
        return count;
    }

    point_count_t cnt(0);

    PointRef point(view->point(0));
    for (PointId idx = 0; idx < num; ++idx)
    {
        point.setPointId(idx);
        processOne(point);
        cnt++;
    }
    return cnt;
}

bool TIndexReader::processOne(PointRef& point)
{
    if (!m_useRustReader)
        return true;
    if (m_rustIndex >= pdal_point_view_length(m_rustView))
        return false;
    copyRustPoint(point, m_rustIndex);
    ++m_rustIndex;
    return true;
}

void TIndexReader::prepared(PointTableRef table)
{
    if (m_useRustReader)
        return;

    m_merge.prepare(table);
    m_merge.setLog(log());
}

void TIndexReader::ready(PointTableRef table)
{
    if (m_useRustReader)
    {
        m_rustIndex = 0;
        return;
    }

    StageWrapper::ready(m_merge, table);
}

PointViewSet TIndexReader::run(PointViewPtr view)
{
    if (m_useRustReader)
    {
        PointViewSet viewSet;
        read(view, m_count);
        viewSet.insert(view);
        return viewSet;
    }

    return StageWrapper::run(m_merge, view);
}

bool TIndexReader::canUseRustReader() const
{
    const bool jsonIndex =
        hasSuffix(m_filename, ".json") || hasSuffix(m_filename, ".geojson");
    if (jsonIndex && (!m_args->m_attributeFilter.empty() ||
                      !m_args->m_sql.empty()))
        return false;
    if (!m_args->m_wkt.empty() && !m_args->m_filterSRS.empty())
        return false;

    return m_args->m_srsColumnName.empty() && m_args->m_ogr.empty() &&
           m_args->m_rawReaderArgs.empty() && m_args->m_tgtSrsString.empty();
}

void TIndexReader::loadRustView()
{
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }
    m_rustIndex = 0;

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);
    addOption(options, "lyr_name", m_args->m_layerName);
    addOption(options, "tindex_name", m_args->m_tileIndexColumnName);
    addOption(options, "where", m_args->m_attributeFilter);
    addOption(options, "sql", m_args->m_sql);
    addOption(options, "dialect", m_args->m_dialect);
    addOption(options, "polygon", m_args->m_wkt);
    if (!m_args->m_bounds.empty())
    {
        std::ostringstream bounds;
        bounds << m_args->m_bounds;
        addOption(options, "bounds", bounds.str());
    }

    pdal_reader_t* reader = pdal_reader_create_tindex(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust TIndex reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError("Rust TIndex reader failed.");
}

void TIndexReader::copyRustPoint(PointRef& point, PointId rustIndex)
{
    for (size_t dimIdx = 0; dimIdx < m_rustDims.size(); ++dimIdx)
    {
        point.setField(m_rustDims[dimIdx],
                       pdal_point_view_get_f64(m_rustView, rustIndex,
                                               m_rustDimNames[dimIdx].c_str()));
    }
}

} // namespace pdal
