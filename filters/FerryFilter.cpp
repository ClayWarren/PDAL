/******************************************************************************
 * Copyright (c) 2014, Howard Butler, howard@hobu.co
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

#include "FerryFilter.hpp"

#include <pdal/util/Algorithm.hpp>
#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>

#include <pdal/private/RustViewConverter.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.ferry", "Copy data from one dimension to another.",
    "https://pdal.org/stages/filters.ferry.html"};

CREATE_STATIC_STAGE(FerryFilter, s_info)

FerryFilter::FerryFilter() : m_rust_stage(nullptr) {}

FerryFilter::~FerryFilter()
{
    pdal_stage_destroy(m_rust_stage);
}

std::string FerryFilter::getName() const
{
    return s_info.name;
}

void FerryFilter::addArgs(ProgramArgs& args)
{
    args.add("dimensions", "List of dimensions to ferry", m_dimSpec)
        .setPositional();
}

void FerryFilter::initialize()
{
    std::vector<const char*> specs;
    specs.reserve(m_dimSpec.size());
    for (const std::string& dim : m_dimSpec)
        specs.push_back(dim.c_str());

    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);

    m_rust_stage = pdal_stage_create_ferry_specs(
        specs.empty() ? nullptr : specs.data(), specs.size());
    if (!m_rust_stage)
        rust_view_converter::throwLastError(
            "Failed to create Rust ferry stage.");

    m_dims.clear();
    for (const std::string& dim : m_dimSpec)
    {
        StringList s = Utils::split(dim, '=');
        if (s.size() != 2)
            throwError("Invalid dimension specified '" + dim +
                       "'. Need <from dimension>=><to dimension>.");
        if (s[1][0] == '>')
            s[1].erase(s[1].begin());
        Utils::trim(s[0]);
        Utils::trim(s[1]);
        m_dims.emplace_back(s[0], s[1]);
    }
}

void FerryFilter::addDimensions(PointLayoutPtr layout)
{
    m_layout = layout;
    for (auto& info : m_dims)
    {
        const Dimension::Id fromId = layout->findDim(info.m_fromName);
        // Dimensions being created with the "=>Dim" syntax won't
        // be in the layout, so we have to assign a default type.
        Dimension::Type fromType = layout->dimType(fromId);
        if (fromType == Dimension::Type::None)
            fromType = Dimension::Type::Double;

        info.m_toId = layout->registerOrAssignDim(info.m_toName, fromType);
    }
}

void FerryFilter::prepared(PointTableRef table)
{
    for (auto& info : m_dims)
    {
        info.m_fromId = table.layout()->findDim(info.m_fromName);
        if (info.m_fromId == Dimension::Id::Unknown && info.m_fromName.size())
            throwError("Can't ferry dimension '" + info.m_fromName +
                       "'. "
                       "Dimension doesn't exist.");
    }
}

bool FerryFilter::processOne(PointRef& point)
{
    if (!m_rust_stage)
        throwError("Rust ferry stage was not initialized.");

    pdal_point_view_t* rustView =
        rust_view_converter::toRustPoint(point, m_layout);
    pdal_stage_ferry_point(m_rust_stage, rustView, 0);
    rust_view_converter::fromRustPoint(rustView, 0, point);
    pdal_point_view_destroy(rustView);
    if (rust_view_converter::hasLastError())
        rust_view_converter::throwLastError("Rust ferry stage failed.");
    return true;
}

PointViewSet FerryFilter::run(PointViewPtr inView)
{
    PointViewSet viewSet;
    viewSet.insert(rust_view_converter::runSingle(m_rust_stage, inView));
    return viewSet;
}

void FerryFilter::filter(PointView& view)
{
    PointRef point(view, 0);
    for (PointId id = 0; id < view.size(); ++id)
    {
        point.setPointId(id);
        processOne(point);
    }
}

} // namespace pdal
