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
#include "private/straighten/Polyline.hpp"

#include <pdal/pdal_internal.hpp>
#include <pdal_capi.h>

namespace pdal
{

static PluginInfo const s_info{"filters.straighten", "Straighten filter",
                               "http://link/to/documentation"};

CREATE_STATIC_STAGE(StraightenFilter, s_info)

struct StraightenFilter::Args
{
public:
    straighten::Polyline m_polyline;
    bool m_unstraighten;
    double m_offset;
};

StraightenFilter::StraightenFilter()
    : Filter(), Streamable(), m_args(new StraightenFilter::Args)
{
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
             m_args->m_polyline)
        .setErrorText("Invalid polyline specification. "
                      "Must be valid GeoJSON/WKT");
    args.add("reverse", "Set to true if you the to unstraighten.",
             m_args->m_unstraighten, false);
    args.add("offset",
             "Use a global offset, so that straighten X starts with that value",
             m_args->m_offset, 0.0);
}

void StraightenFilter::initialize()
{
    if (!m_args->m_polyline.valid())
        throwError("Geometrically invalid polygon in option 'polyline'.");
}

bool StraightenFilter::processOne(PointRef& point)
{

    double segmentX, segmentY, segmentZ, segmentM, segmentAzimuth,
        segmentOffset;
    if (m_args->m_unstraighten)
    {
        m_args->m_polyline.interpolate(point, segmentX, segmentY, segmentZ,
                                       segmentM, segmentAzimuth, segmentOffset);

        double out[3];
        const bool ok = pdal_straighten_transform_point(
            true, point.getFieldAs<double>(Dimension::Id::X),
            point.getFieldAs<double>(Dimension::Id::Y),
            point.getFieldAs<double>(Dimension::Id::Z), segmentX, segmentY,
            segmentZ, segmentM, segmentAzimuth, segmentOffset, m_args->m_offset,
            out);
        if (!ok)
            throwError("Failed to run Rust straighten transform.");

        point.setField(Dimension::Id::X, out[0]);
        point.setField(Dimension::Id::Y, out[1]);
        point.setField(Dimension::Id::Z, out[2]);
        return true;
    }
    else
    {

        if (m_args->m_polyline.closestSegment(
                point, segmentX, segmentY, segmentZ, segmentM, segmentAzimuth,
                segmentOffset) >= 0.0)
        {
            double out[3];
            const bool ok = pdal_straighten_transform_point(
                false, point.getFieldAs<double>(Dimension::Id::X),
                point.getFieldAs<double>(Dimension::Id::Y),
                point.getFieldAs<double>(Dimension::Id::Z), segmentX, segmentY,
                segmentZ, segmentM, segmentAzimuth, segmentOffset,
                m_args->m_offset, out);
            if (!ok)
                throwError("Failed to run Rust straighten transform.");

            point.setField(Dimension::Id::X, out[0]);
            point.setField(Dimension::Id::Y, out[1]);
            point.setField(Dimension::Id::Z, out[2]);
            return true;
        }
    }
    return false;
}

void StraightenFilter::filter(PointView& view)
{
    PointRef point(view, 0);
    for (PointId idx = 0; idx < view.size(); ++idx)
    {
        point.setPointId(idx);
        processOne(point);
    }
    view.invalidateProducts();
}

} // namespace pdal
