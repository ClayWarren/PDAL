/******************************************************************************
 * Copyright (c) 2023, Guilhem Villemin (guilhem.villemin@altametris.com)
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

#include "StraightenFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>
#include <pdal/util/ProgramArgs.hpp>

#include <pdal_capi.h>

namespace pdal
{

static PluginInfo const s_info{"filters.straighten", "Straighten filter",
                               "http://link/to/documentation"};

CREATE_STATIC_STAGE(StraightenFilter, s_info)

struct StraightenFilter::Args
{
    std::string m_polyline;
    bool m_unstraighten;
    double m_offset;
};

StraightenFilter::StraightenFilter()
    : Filter(), Streamable(), m_args(new StraightenFilter::Args)
{
}

StraightenFilter::~StraightenFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

std::string StraightenFilter::getName() const
{
    return s_info.name;
}

void StraightenFilter::addArgs(ProgramArgs& args)
{
    args.add("polyline",
             "Track polyline to straigthen, LineStringZM, with m value is roll "
             "in radians",
             m_args->m_polyline);
    args.add("reverse", "Set to true if you the to unstraighten.",
             m_args->m_unstraighten, false);
    args.add("offset",
             "Use a global offset, so that straighten X starts with that value",
             m_args->m_offset, 0.0);
}

void StraightenFilter::initialize()
{
    // The straightening transform runs through the Rust C ABI; a null stage
    // means the polyline could not be parsed as a LINESTRING ZM.
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);

    m_rust_stage = pdal_stage_create_straighten(
        m_args->m_polyline.c_str(), m_args->m_unstraighten, m_args->m_offset);
    if (!m_rust_stage)
        throwError("Geometrically invalid polyline in option 'polyline'.");
}

void StraightenFilter::prepared(PointTableRef table)
{
    m_layout = table.layout();
}

void StraightenFilter::ready(PointTableRef)
{
    if (m_rust_stage)
        pdal_stage_reset(m_rust_stage);
}

bool StraightenFilter::processOne(PointRef& point)
{
    pdal_point_view_t* rustPoint =
        rust_view_converter::toRustPoint(point, m_layout);
    bool keep = pdal_stage_process_one_at(m_rust_stage, rustPoint, 0);
    if (keep)
        rust_view_converter::fromRustPoint(rustPoint, 0, point);
    pdal_point_view_destroy(rustPoint);
    return keep;
}

void StraightenFilter::filter(PointView& view)
{
    rust_view_converter::runInPlace(m_rust_stage, view);
    view.invalidateProducts();
}

} // namespace pdal
