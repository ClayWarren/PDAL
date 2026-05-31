/******************************************************************************
 * Copyright (c) 2013, Andrew Bell (andrew.bell.ia@gmail.com)
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

#include "HexBinFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>
#include <pdal_capi.h>

#include "private/RustMetadata.hpp"

#include <pdal/Polygon.hpp>
#include <pdal/util/Utils.hpp>

namespace pdal
{

static PluginInfo const s_info =
    PluginInfo("filters.hexbin",
               "Tessellate the point's X/Y domain and determine point density "
               "and/or point boundary.",
               "https://pdal.org/stages/filters.hexbin.html");

CREATE_STATIC_STAGE(HexBin, s_info)

HexBin::HexBin() : m_rustStage(nullptr), m_usedRust(false) {}

HexBin::~HexBin()
{
    if (m_rustStage)
        pdal_stage_destroy(m_rustStage);
}

std::string HexBin::getName() const
{
    return s_info.name;
}

hexer::BaseGrid* HexBin::grid() const
{
    return nullptr;
}

void HexBin::addArgs(ProgramArgs& args)
{
    args.add("sample_size",
             "Maximum sample size for auto-edge length calculation",
             m_sampleSize, 5000U);
    args.add("threshold", "Required cell density", m_density, 15);
    args.add("output_tesselation", "Write tesselation to output metadata",
             m_outputTesselation);
    args.add("edge_size", "Synonym for 'edge_length' (deprecated)",
             m_edgeLength);
    args.add("edge_length", "Length of hex edge", m_edgeLength);
    args.add("precision", "Output precision", m_precision, 8U);
    m_cullArg = &args.add("hole_cull_area_tolerance",
                          "Tolerance area to "
                          "apply to holes before cull",
                          m_cullArea);
    args.add("smooth", "Smooth boundary output", m_doSmooth, true);
    args.add("preserve_topology", "Preserve topology when smoothing",
             m_preserve_topology, true);
    args.add("density",
             "Emit a density tessellation to a specified OGR-compatible output "
             "file. "
             "Defaults to GeoJSON unless 'ogrdriver' option is set.",
             m_DensityOutput, "");
    args.add("boundary",
             "Emit a boundary tessellation to a specified OGR-compatible "
             "output file. "
             "Defaults to GeoJSON unless 'ogrdriver' option is set.",
             m_boundaryOutput, "");
    args.add("h3_grid",
             "Create a grid using H3 (https://h3geo.org/docs) Hexagons", m_isH3,
             false);
    args.add("h3_resolution",
             "H3 grid resolution: 0 (coarsest) - 15 (finest). See "
             "https://h3geo.org/docs/core-library/restable",
             m_h3Res, -1);
    args.add("ogrdriver",
             "GDAL OGR vector driver for writing with 'density' or 'boundary' "
             "options.",
             m_driver, "GeoJSON");
}

bool HexBin::useRustPath() const
{
    return true;
}

pdal_stage* HexBin::createRustStage()
{
    pdal_options_t* ops = pdal_options_create();
    if (m_edgeLength > 0)
        pdal_options_add_f64(ops, "edge_length", m_edgeLength);
    pdal_options_add_u64(ops, "threshold", m_density);
    pdal_options_add_u64(ops, "sample_size", m_sampleSize);
    pdal_options_add_str(ops, "output_tesselation",
                         m_outputTesselation ? "true" : "false");
    if (!m_DensityOutput.empty())
        pdal_options_add_str(ops, "density", m_DensityOutput.c_str());
    if (!m_boundaryOutput.empty())
        pdal_options_add_str(ops, "boundary", m_boundaryOutput.c_str());
    pdal_options_add_str(ops, "ogrdriver", m_driver.c_str());
    pdal_options_add_str(ops, "lyr_name", "hexbins");
    if (m_isH3)
    {
        pdal_options_add_str(ops, "h3_grid", "true");
        if (m_h3Res != -1)
            pdal_options_add_u64(ops, "h3_resolution", m_h3Res);
    }

    pdal_stage* stage = pdal_stage_create_hexbin(ops);
    pdal_options_destroy(ops);
    if (!stage)
        throwError("Failed to create Rust hexbin stage.");
    return stage;
}

PointViewSet HexBin::run(PointViewPtr view)
{
    if (useRustPath() && !view->empty())
    {
        if (m_rustStage)
            pdal_stage_destroy(m_rustStage);
        m_rustStage = createRustStage();
        m_usedRust = true;
        rust_view_converter::runInPlace(m_rustStage, *view);
    }

    PointViewSet viewSet;
    viewSet.insert(view);
    return viewSet;
}

void HexBin::initialize()
{
    if (m_isH3)
    {
        if (m_edgeLength)
        {
            if (m_h3Res == -1)
                throwError("'edge_length' not implemented for H3 processing. "
                           "Set 'h3_resolution' option to specify cell size.");
            else
                log()->get(LogLevel::Warning)
                    << "'edge_length' not implemented "
                       "for H3 processing. Using 'h3_resolution'\n";
        }
        if (m_cullArea || !m_preserve_topology)
            throwError("Smoothing not implemented for H3 processing. "
                       "'preserve_topology' and 'hole_cull_area_tolerance' "
                       "options are ignored.");
    }
    if (!m_isH3 && (m_h3Res != -1))
    {
        if (!m_edgeLength)
            throwError(
                "'h3_resolution' not implemented for standard "
                "processing. Set 'edge_length' option to specify cell size.");
        else
            log()->get(LogLevel::Warning)
                << "'h3_resolution' not implemented "
                   "for standard processing. Using 'edge_length'\n";
    }
}

void HexBin::ready(PointTableRef table)
{
    m_count = 0;
    m_streamView.reset(new PointView(table, m_srs));
}

bool HexBin::processOne(PointRef& point)
{
    PointId idx = m_streamView->size();
    m_streamView->point(idx);
    for (Dimension::Id dim : point.layout()->dims())
        m_streamView->setField(dim, idx, point.getFieldAs<double>(dim));
    m_count++;
    return true;
}

void HexBin::spatialReferenceChanged(const SpatialReference& srs)
{
    m_srs = srs;
}

void HexBin::addRustMetadata(PointTableRef table)
{
    pdal_metadata_node_t* rustMetadata = pdal_stage_metadata(m_rustStage);
    if (rustMetadata)
    {
        rust_metadata::addChildrenTo(m_metadata, rustMetadata);
        pdal_metadata_node_destroy(rustMetadata);
    }

    MetadataNode rawNode = m_metadata.findChild("hex_boundary_raw");
    std::string rawWkt = rawNode.valid() ? rawNode.value() : std::string();
    MetadataNode heightNode = m_metadata.findChild("estimated_edge");
    double gridHeight = heightNode.valid() ? heightNode.value<double>() : 0.0;

    pdal::Polygon p(rawWkt.empty() ? std::string("MULTIPOLYGON EMPTY") : rawWkt,
                    m_srs);
    if (m_doSmooth && gridHeight > 0.0 && !rawWkt.empty() &&
        rawWkt != "MULTIPOLYGON EMPTY")
    {
        double tolerance = 1.1 * gridHeight / 2.0;
        double cull =
            m_cullArg->set() ? m_cullArea : (6 * tolerance * tolerance);
        p.simplify(tolerance, cull, m_preserve_topology);
    }

    m_metadata.add("boundary", p.wkt(m_precision),
                   "Approximated MULTIPOLYGON of domain");
    m_metadata.addWithType("boundary_json", p.json(), "json",
                           "Approximated MULTIPOLYGON of domain");
}

void HexBin::done(PointTableRef table)
{
    if (!m_usedRust && m_streamView && !m_streamView->empty())
    {
        if (m_rustStage)
            pdal_stage_destroy(m_rustStage);
        m_rustStage = createRustStage();
        m_usedRust = true;
        rust_view_converter::runInPlace(m_rustStage, *m_streamView);
    }

    if (m_usedRust)
    {
        addRustMetadata(table);
        if (m_rustStage)
        {
            pdal_stage_destroy(m_rustStage);
            m_rustStage = nullptr;
        }
        m_usedRust = false;
    }
    m_streamView.reset();
}

} // namespace pdal
