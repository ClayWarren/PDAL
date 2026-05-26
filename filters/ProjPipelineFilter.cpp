/******************************************************************************
 * Copyright (c) 2019, Aurelien Vila (aurelien.vila@delair.aero)
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

#include "ProjPipelineFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>

#include <pdal/PointView.hpp>
#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.projpipeline",
    "Transform coordinates using Proj pipeline string, WKT2 coordinate "
    "operations or URN definition",
    "https://pdal.org/stages/filters.projpipeline.html"};

CREATE_STATIC_STAGE(ProjPipelineFilter, s_info)

std::string ProjPipelineFilter::getName() const
{
    return s_info.name;
}

ProjPipelineFilter::ProjPipelineFilter() {}

ProjPipelineFilter::~ProjPipelineFilter() {}

void ProjPipelineFilter::addArgs(ProgramArgs& args)
{
    args.add("out_srs", "Output spatial reference", m_outSRS);
    args.add("reverse_transfo",
             "Wether the coordinate operation should be evaluated in the "
             "reverse path",
             m_reverseTransfo, false);
    args.add("coord_op",
             "Coordinate operation (Proj pipeline or WKT2 string or urn "
             "definition)",
             m_coordOperation)
        .setPositional();
}

void ProjPipelineFilter::initialize()
{
    setSpatialReference(m_outSRS);
}

void ProjPipelineFilter::prepared(PointTableRef table)
{
    m_layout = table.layout();
}

PointViewSet ProjPipelineFilter::run(PointViewPtr view)
{
    PointViewSet viewSet;

    pdal_stage_t* stage = pdal_stage_create_projpipeline(
        m_outSRS.getWKT().c_str(), m_coordOperation.c_str(), m_reverseTransfo);
    if (!stage)
    {
        const char* message = pdal_last_error();
        if (message && message[0])
            throwError(std::string("filters.projpipeline: ") + message);
        throwError("Failed to create Rust projpipeline stage.");
    }

    PointViewPtr outView = rust_view_converter::runSingle(stage, view);
    pdal_stage_destroy(stage);
    viewSet.insert(outView);
    return viewSet;
}

bool ProjPipelineFilter::processOne(PointRef& point)
{
    pdal_stage_t* stage = pdal_stage_create_projpipeline(
        m_outSRS.getWKT().c_str(), m_coordOperation.c_str(), m_reverseTransfo);
    if (!stage)
        rust_view_converter::throwLastError(
            "Failed to create Rust projpipeline stage.");

    pdal_point_view_t* rustView =
        rust_view_converter::toRustPoint(point, m_layout);
    bool ok = pdal_stage_process_one_at(stage, rustView, 0);
    rust_view_converter::fromRustPoint(rustView, 0, point);
    pdal_point_view_destroy(rustView);
    pdal_stage_destroy(stage);
    if (rust_view_converter::hasLastError())
        rust_view_converter::throwLastError("Rust projpipeline failed.");
    return ok;
}

} // namespace pdal
