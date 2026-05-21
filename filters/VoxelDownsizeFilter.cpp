/******************************************************************************
 * Copyright (c) 2019, Helix.re
 * Contact Person : Pravin Shinde (pravin@helix.re,
 *                    https://github.com/pravinshinde825)
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
 *
 ****************************************************************************/

#include "VoxelDownsizeFilter.hpp"
#include <pdal/private/RustViewConverter.hpp>

#include <pdal_capi.h>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.voxeldownsize", "First Entry Voxel Filter",
    "https://pdal.org/stages/filters.voxeldownsize.html"};

CREATE_STATIC_STAGE(VoxelDownsizeFilter, s_info)

std::istream& operator>>(std::istream& in, VoxelDownsizeFilter::Mode& mode)
{
    std::string s;
    in >> s;

    s = Utils::tolower(s);
    if (s == "center")
        mode = VoxelDownsizeFilter::Mode::Center;
    else if (s == "first")
        mode = VoxelDownsizeFilter::Mode::First;
    else
        throw pdal_error("filters.voxeldownsize: Invalid 'mode' option '" + s +
                         "'. "
                         "Valid options are 'center' and 'first'");
    return in;
}

std::ostream& operator<<(std::ostream& out,
                         const VoxelDownsizeFilter::Mode& mode)
{
    switch (mode)
    {
    case VoxelDownsizeFilter::Mode::Center:
        out << "center";
        break;
    case VoxelDownsizeFilter::Mode::First:
        out << "first";
        break;
    }
    return out;
}

VoxelDownsizeFilter::VoxelDownsizeFilter() : m_rustStage(nullptr) {}

VoxelDownsizeFilter::~VoxelDownsizeFilter()
{
    pdal_stage_destroy(m_rustStage);
}

std::string VoxelDownsizeFilter::getName() const
{
    return s_info.name;
}

void VoxelDownsizeFilter::addArgs(ProgramArgs& args)
{
    args.add("cell", "Cell size", m_cell, 0.001);
    args.add("mode", "Method for downsizing : center / first", m_mode,
             Mode::Center);
}

void VoxelDownsizeFilter::initialize()
{
    pdal_options_t* ops = pdal_options_create();
    pdal_options_add_f64(ops, "cell", m_cell);
    pdal_options_add_str(ops, "mode",
                         m_mode == Mode::Center ? "center" : "first");

    if (m_rustStage)
        pdal_stage_destroy(m_rustStage);

    m_rustStage = pdal_stage_create_voxeldownsize(ops);
    pdal_options_destroy(ops);
    if (!m_rustStage)
        throwError("Failed to create Rust voxeldownsize stage.");
}

void VoxelDownsizeFilter::ready(PointTableRef table)
{
    m_layout = table.layout();
    if (m_rustStage)
        pdal_stage_reset(m_rustStage);
}

PointViewSet VoxelDownsizeFilter::run(PointViewPtr view)
{
    PointViewSet viewSet;
    viewSet.insert(rust_view_converter::runSingle(m_rustStage, view));
    return viewSet;
}

bool VoxelDownsizeFilter::processOne(PointRef& point)
{
    if (!m_rustStage)
        throwError("Rust voxeldownsize stage was not initialized.");

    pdal_point_view_t* rustView =
        rust_view_converter::toRustPoint(point, m_layout);
    bool keep = pdal_stage_process_one_at(m_rustStage, rustView, 0);
    rust_view_converter::fromRustPoint(rustView, 0, point);
    pdal_point_view_destroy(rustView);
    if (rust_view_converter::hasLastError())
        rust_view_converter::throwLastError(
            "Rust voxeldownsize streaming failed.");
    return keep;
}

} // namespace pdal
