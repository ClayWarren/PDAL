/******************************************************************************
 * Copyright (c) 2017, Hobu Inc. <info@hobu.co>
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
 *     * Neither the name of Hobu, Inc. nor the names of its contributors
 *       may be used to endorse or promote products derived from this
 *       software without specific prior written permission.
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

// Add compatibility for deprecated integer data types no longer available
// by default in GDAL 4.0 (GIntBig)
#define GDAL_USE_OLD_INT_TYPES

#include "OGRWriter.hpp"

#include <algorithm>
#include <sstream>
#include <tuple>

#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wredundant-decls"
#include <pdal/PointView.hpp>
#include <pdal/private/RustViewConverter.hpp>
#include <pdal/private/gdal/ErrorHandler.hpp>
#include <pdal/private/gdal/GDALUtils.hpp>
#include <pdal/util/FileUtils.hpp>

#include <ogr_core.h>
#include <ogrsf_frmts.h>
#pragma GCC diagnostic pop

#include <pdal_capi.h>

namespace pdal
{

namespace
{

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
    pdal_options_add_str(options, key.c_str(), value.c_str());
}

void addOption(pdal_options_t* options, const std::string& key, size_t value)
{
    pdal_options_add_u64(options, key.c_str(), value);
}

bool rustSupportedOgrOption(const std::string& value)
{
    return value == "WRITE_BBOX=YES" ||
           value.rfind("COORDINATE_PRECISION=", 0) == 0 ||
           value == "RFC7946=YES";
}

} // unnamed namespace

static StaticPluginInfo const s_info{
    "writers.ogr",
    "Write a point cloud as a set of OGR points/multipoints",
    "https://pdal.org/stages/writers.ogr.html",
    {"shp", "geojson"}};

CREATE_STATIC_STAGE(OGRWriter, s_info)

OGRWriter::OGRWriter()
    : m_driver(nullptr), m_ds(nullptr), m_layer(nullptr), m_feature(nullptr),
      m_curCount(0), m_measureDim(Dimension::Id::Unknown),
      m_inTransaction(false)
{
}

OGRWriter::~OGRWriter()
{
    clearRustViews();
}

std::string OGRWriter::getName() const
{
    return s_info.name;
}

void OGRWriter::addArgs(ProgramArgs& args)
{
    args.add("multicount", "Group 'multicount' points into a structure",
             m_multiCount, (size_t)1);
    args.add("measure_dim", "Use dimensions as a measure value",
             m_measureDimName);
    args.add("ogrdriver", "OGR writer driver name", m_driverName, m_driverName);
    args.add("ogr_options", "OGR layer creation options", m_ogrOptions);
    args.add("attr_dims",
             "Dimension to use as attributes, 'all' for all. "
             "Incompatible with multicount>1",
             m_attrDimNames);
}

void OGRWriter::initialize()
{
    gdal::registerDrivers();
    char* err =
        pdal_ogr_writer_validate(static_cast<uint64_t>(m_multiCount),
                                 static_cast<uint64_t>(m_attrDimNames.size()));
    if (err)
    {
        std::string message(err);
        pdal_string_free(err);
        throwError(message);
    }
}

void OGRWriter::prepared(PointTableRef table)
{
    if (m_measureDimName.size())
    {
        m_measureDim = table.layout()->findDim(m_measureDimName);
        if (m_measureDim == Dimension::Id::Unknown)
            throwError("Dimension '" + m_measureDimName +
                       "' (measure_dim) not "
                       "found.");
    }

    if (m_driverName.empty())
    {
        if (FileUtils::extension(filename()) == ".geojson")
            m_driverName = "GeoJSON";
        else
            m_driverName = "ESRI Shapefile";
    }

    // Build the attr dims list, replacing special keywords with the proper
    // field names.
    for (auto& name : m_attrDimNames)
    {
        if (name == "all")
        {
            m_attrDimNames.clear();
            for (auto& dim : table.layout()->dims())
            {
                switch (dim)
                {
                // we don't need geometry attributes repeated as fields
                case Dimension::Id::X:
                case Dimension::Id::Y:
                case Dimension::Id::Z:
                    break;

                default:
                    if (dim != m_measureDim)
                    {
                        m_attrDimNames.push_back(table.layout()->dimName(dim));
                    }
                }
            }
            break;
        }
        else
        {
            auto dim = table.layout()->findDim(name);
            if (dim == Dimension::Id::Unknown)
            {
                char* err = pdal_ogr_writer_dim_not_found(name.c_str());
                std::string message =
                    err ? std::string(err)
                        : "Dimension '" + name + "' (attr_dims) not found.";
                pdal_string_free(err);
                throwError(message);
            }
        }
    }
}

void OGRWriter::readyTable(PointTableRef table)
{
    m_driver = GetGDALDriverManager()->GetDriverByName(m_driverName.data());
    m_geomType = (m_multiCount == 1) ? wkbPointZM : wkbMultiPointZM;

    const auto& layout = table.layout();
    for (auto& name : m_attrDimNames)
    {
        auto dim = layout->findDim(name);
        auto dimType = layout->dimType(dim);
        OGRFieldType ogrType;

        switch (dimType)
        {
        case Dimension::Type::Signed8:
        case Dimension::Type::Unsigned8:
        case Dimension::Type::Signed16:
        case Dimension::Type::Unsigned16:
        case Dimension::Type::Signed32:
            ogrType = OFTInteger;
            break;
        case Dimension::Type::Unsigned32:
        case Dimension::Type::Signed64:
        case Dimension::Type::Unsigned64: // error here?
            ogrType = OFTInteger64;
            break;
        case Dimension::Type::Float:
        case Dimension::Type::Double:
            ogrType = OFTReal;
            break;
        case Dimension::Type::None:
        default:
            throwError("Unknown type for dimension '" + name +
                       "' (attr_dims).");
            continue;
        }

        // This is strange code. The attributes stored in m_attrs are a tuple
        // and the third element is an OGRFieldDefn, NOT an OGRFieldDefn*.
        // However, there is a constructor for an OGRFieldDefn that takes
        // OGRFieldDefn* and that's what's invoked in emplace_back() below.
        // Despite the existince of this copying via a pointer, older versions
        // of GDAL disallowed the normal copy constructor for OGRFieldDefn. This
        // changed with GDAL version 3.10.2, where regular copy ctors were
        // enabled. So if PDAL requries GDAL of at least that version, this
        // dynamic allocation can be replaced with a stack-based construction.
        std::unique_ptr<OGRFieldDefn> fieldDef(
            new OGRFieldDefn(name.c_str(), ogrType));
        m_attrs.emplace_back(dim, dimType, fieldDef.get());
    }
}

void OGRWriter::readyFile(const std::string& filename,
                          const SpatialReference& srs)
{
    m_curCount = 0;
    m_outputFilename = filename;
    m_outputSrsWkt = srs.getWKT();

    m_rustWriter = useRustWriter();
    clearRustViews();
    if (m_rustWriter)
        return;

    // Dataset
    m_ds = m_driver->Create(filename.data(), 0, 0, 0, GDT_Unknown, nullptr);
    if (!m_ds)
        throwError("Unable to open OGR datasource '" + filename +
                   "': " + CPLGetLastErrorMsg());

    // CRS
    if (!srs.empty())
    {
        if (m_srs.importFromWkt(srs.getWKT().data()) != OGRERR_NONE)
            throwError(std::string("Can't initialise OGR SRS: ") +
                       CPLGetLastErrorMsg());
    }

    // Creation options
    std::vector<const char*> ogr_create_options;
    ogr_create_options.reserve(m_ogrOptions.size());
    for (auto&& o : m_ogrOptions)
        ogr_create_options.push_back(o.c_str());
    ogr_create_options.push_back(nullptr);

    // Layer
    m_layer = m_ds->CreateLayer("points", &m_srs, m_geomType,
                                const_cast<char**>(ogr_create_options.data()));
    if (!m_layer)
        throwError(std::string("Can't create OGR layer: ") +
                   CPLGetLastErrorMsg());

    // Fields
    for (auto& attr : m_attrs)
    {
        auto& ogrField = std::get<2>(attr);
        if (m_layer->CreateField(&ogrField) != OGRERR_NONE)
            throwError(std::string("Can't create OGR field: ") +
                       ogrField.GetNameRef());
    }

    // Reusable template feature
    m_feature = OGRFeature::CreateFeature(m_layer->GetLayerDefn());
    if (!m_feature)
        throwError(std::string("Can't create template OGR feature: ") +
                   CPLGetLastErrorMsg());

    // Try to use a transaction for data sources that support it (e.g. GPKG),
    // otherwise new points may get auto-committed after each insert (very slow)
    if (m_ds->TestCapability(ODsCTransactions) &&
        m_ds->StartTransaction() == OGRERR_NONE)
        m_inTransaction = true;
}

void OGRWriter::writeView(const PointViewPtr view)
{
    if (m_rustWriter)
    {
        m_rustViews.push_back(rust_view_converter::toRust(view));
        return;
    }

    m_curCount = 0;
    PointRef point(*view, 0);
    for (PointId idx = 0; idx < view->size(); ++idx)
    {
        point.setPointId(idx);
        processOne(point);
    }
}

bool OGRWriter::processOne(PointRef& point)
{
    double x = point.getFieldAs<double>(Dimension::Id::X);
    double y = point.getFieldAs<double>(Dimension::Id::Y);
    double z = point.getFieldAs<double>(Dimension::Id::Z);
    double m = point.getFieldAs<double>(m_measureDim);

    OGRPoint pt(x, y, z);
    if (m_measureDim != Dimension::Id::Unknown)
        pt.setM(m);

    m_curCount++;

    if (m_multiCount > 1)
        m_multiPoint.addGeometry(&pt);

    if (m_curCount % m_multiCount == 0)
    {
        if (m_multiCount > 1)
        {
            m_feature->SetGeometry(&m_multiPoint);
            m_multiPoint.empty();
        }
        else
        {
            m_feature->SetGeometry(&pt);

            for (auto it = std::begin(m_attrs); it != std::end(m_attrs); ++it)
            {
                const auto& dim = std::get<0>(*it);
                const auto& dimType = std::get<1>(*it);
                const auto& ogrField = std::get<2>(*it);
                size_t ogr_field_idx = std::distance(std::begin(m_attrs), it);

                switch (dimType)
                {
                case Dimension::Type::Signed8:
                case Dimension::Type::Unsigned8:
                case Dimension::Type::Signed16:
                case Dimension::Type::Unsigned16:
                case Dimension::Type::Signed32:
                    m_feature->SetField(ogr_field_idx,
                                        point.getFieldAs<int>(dim));
                    break;

                case Dimension::Type::Unsigned32:
                case Dimension::Type::Unsigned64:
                case Dimension::Type::Signed64:
                    m_feature->SetField(ogr_field_idx,
                                        point.getFieldAs<GIntBig>(dim));
                    break;

                case Dimension::Type::Float:
                case Dimension::Type::Double:
                    m_feature->SetField(ogr_field_idx,
                                        point.getFieldAs<double>(dim));
                    break;

                default:
                    break;
                }
            }
        }

        if (m_layer->CreateFeature(m_feature))
            throwError(std::string("Can't create OGR feature: ") +
                       CPLGetLastErrorMsg());

        m_feature->Reset();
    }
    return true;
}

void OGRWriter::doneFile()
{
    if (m_rustWriter)
    {
        writeRustOutput();
        clearRustViews();
        getMetadata().addList("filename", filename());
        return;
    }

    if (m_curCount % m_multiCount > 0)
    {
        m_feature->Reset();

        m_feature->SetGeometry(&m_multiPoint);

        if (m_layer->CreateFeature(m_feature))
            throwError(std::string("Can't create OGR feature: ") +
                       CPLGetLastErrorMsg());
    }
    OGRFeature::DestroyFeature(m_feature);

    if (m_inTransaction && m_ds->CommitTransaction() != OGRERR_NONE)
        throwError(std::string("Failed to commit transaction in OGR: ") +
                   CPLGetLastErrorMsg());
    m_inTransaction = false;

    GDALClose(m_ds);
    m_layer = nullptr;
    m_ds = nullptr;
}

bool OGRWriter::useRustWriter() const
{
    if (m_driverName == "GeoJSON")
        return m_measureDimName.empty() &&
               std::all_of(m_ogrOptions.begin(), m_ogrOptions.end(),
                           rustSupportedOgrOption);

    return (m_driverName == "ESRI Shapefile" && m_ogrOptions.empty()) ||
           (m_driverName == "GPKG" && m_ogrOptions.empty() &&
            m_multiCount == 1 && m_measureDimName.empty());
}

void OGRWriter::writeRustOutput()
{
    if (m_rustViews.empty())
        return;

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_outputFilename);
    addOption(options, "ogrdriver", m_driverName);
    if (m_multiCount > 1)
        addOption(options, "multicount", m_multiCount);
    for (const auto& option : m_ogrOptions)
        addOption(options, "ogr_options", option);
    if (!m_outputSrsWkt.empty())
        addOption(options, "input_srs", m_outputSrsWkt);
    if (!m_measureDimName.empty())
        addOption(options, "measure_dim", m_measureDimName);
    if (!m_attrDimNames.empty())
    {
        std::string joined;
        for (const auto& name : m_attrDimNames)
        {
            if (!joined.empty())
                joined += ",";
            joined += name;
        }
        addOption(options, "attr_dims", joined);
    }

    pdal_writer_t* writer = pdal_writer_create_ogr(options);
    if (!writer)
    {
        pdal_options_destroy(options);
        rust_view_converter::throwLastError(
            "Failed to create Rust OGR writer.");
    }

    std::vector<const pdal_point_view_t*> rustViews(m_rustViews.begin(),
                                                    m_rustViews.end());
    bool ok =
        pdal_writer_write_views(writer, rustViews.data(), rustViews.size());
    pdal_writer_destroy(writer);
    pdal_options_destroy(options);
    if (!ok)
        rust_view_converter::throwLastError("Rust OGR writer failed.");
}

void OGRWriter::clearRustViews()
{
    for (pdal_point_view_t* view : m_rustViews)
        pdal_point_view_destroy(view);
    m_rustViews.clear();
}

} // namespace pdal
