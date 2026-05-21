/******************************************************************************
 * Copyright (c) 2020, Hobu Inc.
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

#include "FaceRasterFilter.hpp"
#include <pdal/private/RustViewConverter.hpp>

#include <pdal/private/Raster.hpp>
#include <pdal_capi.h>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.faceraster", "Face Raster Filter",
    "https://pdal.org/stages/filters.faceraster.html"};

CREATE_STATIC_STAGE(FaceRasterFilter, s_info)

std::string FaceRasterFilter::getName() const
{
    return s_info.name;
}

FaceRasterFilter::FaceRasterFilter() : m_limits(new RasterLimits) {}

FaceRasterFilter::~FaceRasterFilter() {}

void FaceRasterFilter::addArgs(ProgramArgs& args)
{
    m_limits->addArgs(args);
    args.add("mesh", "Mesh name", m_meshName);
    args.add("nodata", "No data value", m_noData,
             std::numeric_limits<double>::quiet_NaN());
    args.add("max_triangle_edge_length", "Max triangle edge length",
             m_maxTriangleEdgeLength, std::numeric_limits<double>::infinity());
}

void FaceRasterFilter::prepared(PointTableRef)
{
    int cnt = m_limits->checkArgs();
    if (cnt != 0 && cnt != 4)
        throwError("Must specify all or none of 'origin_x', 'origin_y', "
                   "'width' and 'height'.");
    m_computeLimits = (cnt == 0);
}

void FaceRasterFilter::filter(PointView& v)
{
    pdal_options_t* ops = pdal_options_create();
    pdal_options_add_f64(ops, "resolution", m_limits->edgeLength);
    if (!m_computeLimits)
    {
        pdal_options_add_f64(ops, "origin_x", m_limits->xOrigin);
        pdal_options_add_f64(ops, "origin_y", m_limits->yOrigin);
        pdal_options_add_u64(ops, "width", m_limits->width);
        pdal_options_add_u64(ops, "height", m_limits->height);
    }
    if (!m_meshName.empty())
        pdal_options_add_str(ops, "mesh", m_meshName.c_str());
    pdal_options_add_f64(ops, "nodata", m_noData);
    pdal_options_add_f64(ops, "max_triangle_edge_length",
                         m_maxTriangleEdgeLength);

    pdal_stage_t* stage = pdal_stage_create_faceraster(ops);
    if (!stage)
    {
        pdal_options_destroy(ops);
        throwError("Failed to create Rust faceraster stage.");
    }

    rust_view_converter::runInPlace(stage, v);
    pdal_stage_destroy(stage);
    pdal_options_destroy(ops);
}

} // namespace pdal
