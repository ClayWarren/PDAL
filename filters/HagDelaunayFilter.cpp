/******************************************************************************
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

#include "HagDelaunayFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>

#include <pdal_capi.h>

#include <string>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.hag_delaunay",
    "Computes height above ground using delaunay interpolation of "
    "ground returns.",
    "https://pdal.org/stages/filters.hag_delaunay.html"};

CREATE_STATIC_STAGE(HagDelaunayFilter, s_info)

std::string HagDelaunayFilter::getName() const
{
    return s_info.name;
}

HagDelaunayFilter::HagDelaunayFilter() {}

void HagDelaunayFilter::addArgs(ProgramArgs& args)
{
    args.add("count",
             "The number of points to fetch to determine the "
             "ground point [Default: 10].",
             m_count, point_count_t(10));
    args.add("allow_extrapolation",
             "Allow extrapolation for points "
             "outside of the local triangulations. [Default: true].",
             m_allowExtrapolation, true);
    args.add("class", "Class to use for ground points. [Default: 2]", m_class,
             ClassLabel::Ground);
}

void HagDelaunayFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Dimension::Id::HeightAboveGround);
}

void HagDelaunayFilter::prepared(PointTableRef table)
{
    if (m_count < 3)
        throwError("Option 'count' must be at least 3.");

    const PointLayoutPtr layout(table.layout());
    if (!layout->hasDim(Dimension::Id::Classification))
        throwError("Missing Classification dimension in input PointView.");
}

void HagDelaunayFilter::filter(PointView& view)
{
    pdal_stage_t* stage =
        pdal_stage_create_hag_delaunay(m_count, m_allowExtrapolation, m_class);
    if (!stage)
        throwError("Failed to create Rust hag_delaunay stage.");

    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
