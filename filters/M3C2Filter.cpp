/******************************************************************************
 * Copyright (c) 2025, Hobu Inc.
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

#include "M3C2Filter.hpp"

#include <pdal/private/RustViewConverter.hpp>
#include <pdal_capi.h>

namespace pdal
{

struct M3C2Filter::Args
{
    double normalRadius;
    double cylRadius;
    double cylHalfLen;
    double regError;
    NormalOrientation orientation;
    int minPoints;
};

struct M3C2Filter::Private
{
    PointViewPtr v1;
    PointViewPtr v2;
    PointViewPtr cores;
    Dimension::Id distanceDim;
    Dimension::Id uncertaintyDim;
    Dimension::Id significantDim;
    Dimension::Id stdDev1Dim;
    Dimension::Id stdDev2Dim;
    Dimension::Id n1Dim;
    Dimension::Id n2Dim;
};

static StaticPluginInfo const s_info{"filters.m3c2",
                                     "Compute the 3D distance between two sets "
                                     "of points based on the M3C2 algorithm",
                                     "http://pdal.io/stages/filters.m3c2.html"};

CREATE_STATIC_STAGE(M3C2Filter, s_info)

std::string M3C2Filter::getName() const
{
    return s_info.name;
}

M3C2Filter::M3C2Filter()
    : m_args(new M3C2Filter::Args), m_p(new M3C2Filter::Private)
{
}

M3C2Filter::~M3C2Filter() {}

std::istream& operator>>(std::istream& in, M3C2Filter::NormalOrientation& mode)
{
    std::string s;
    in >> s;

    s = Utils::tolower(s);
    if (s == "up")
        mode = M3C2Filter::NormalOrientation::Up;
    else if (s == "down")
        mode = M3C2Filter::NormalOrientation::Down;
    else if (s == "none")
        mode = M3C2Filter::NormalOrientation::None;
    else
        in.setstate(std::ios_base::failbit);
    return in;
}

std::ostream& operator<<(std::ostream& out,
                         const M3C2Filter::NormalOrientation& mode)
{
    switch (mode)
    {
    case M3C2Filter::NormalOrientation::Up:
        out << "up";
        break;
    case M3C2Filter::NormalOrientation::Down:
        out << "down";
        break;
    case M3C2Filter::NormalOrientation::None:
        out << "none";
        break;
    }
    return out;
}

void M3C2Filter::addArgs(ProgramArgs& args)
{
    args.add("normal_radius",
             "The radius to use for finding neighbors in the "
             "calculation of normals [Default: 2].",
             m_args->normalRadius, 2.0);
    args.add(
        "cyl_radius",
        "The radius of the cylinder of neighbors used for calculating change "
        "[Default: 2].",
        m_args->cylRadius, 2.0);
    args.add("cyl_halflen",
             "The half-length of the cylinder of neighbors used used for "
             "calculating change [Default: 5].",
             m_args->cylHalfLen, 5.0);
    args.add("reg_error", "Registration error [Default: 0].", m_args->regError,
             0.0);
    args.add("orientation", "Orientation of the cylinder & normal",
             m_args->orientation, NormalOrientation::Up);
    args.add(
        "min_points",
        "Minimum number of points within a neighborhood to use for calculating "
        "statistics [Default: 1].",
        m_args->minPoints, 1);
}

void M3C2Filter::addDimensions(PointLayoutPtr layout)
{
    m_p->distanceDim =
        layout->assignDim("m3c2_distance", Dimension::Type::Double);
    m_p->uncertaintyDim =
        layout->assignDim("m3c2_uncertainty", Dimension::Type::Double);
    m_p->significantDim =
        layout->assignDim("m3c2_significant", Dimension::Type::Unsigned8);
    m_p->stdDev1Dim =
        layout->assignDim("m3c2_std_dev1", Dimension::Type::Double);
    m_p->stdDev2Dim =
        layout->assignDim("m3c2_std_dev2", Dimension::Type::Double);
    m_p->n1Dim = layout->assignDim("m3c2_count1", Dimension::Type::Unsigned16);
    m_p->n2Dim = layout->assignDim("m3c2_count2", Dimension::Type::Unsigned16);
}

PointViewSet M3C2Filter::run(PointViewPtr view)
{
    if (!m_p->v1)
        m_p->v1 = view;
    else if (!m_p->v2)
        m_p->v2 = view;
    else if (!m_p->cores)
        m_p->cores = view;

    PointViewSet set;
    if (m_p->cores)
        set.insert(m_p->cores);
    return set;
}

void M3C2Filter::done(PointTableRef _)
{
    if (!m_p->v1)
        throwError("Missing first view.");
    if (!m_p->v2)
        throwError("Missing second view.");
    if (!m_p->cores)
        throwError("Missing core points.");

    uint8_t orientation = 0;
    if (m_args->orientation == NormalOrientation::Down)
        orientation = 1;
    else if (m_args->orientation == NormalOrientation::None)
        orientation = 2;

    pdal_point_view_t* rustView1 = rust_view_converter::toRust(m_p->v1);
    pdal_point_view_t* rustView2 = rust_view_converter::toRust(m_p->v2);
    pdal_point_view_t* rustCores = rust_view_converter::toRust(m_p->cores);
    pdal_point_view_t* rustOut =
        pdal_m3c2_compute(rustView1, rustView2, rustCores, m_args->normalRadius,
                          m_args->cylRadius, m_args->cylHalfLen,
                          m_args->regError, orientation, m_args->minPoints);
    pdal_point_view_destroy(rustView1);
    pdal_point_view_destroy(rustView2);
    pdal_point_view_destroy(rustCores);

    if (!rustOut)
        rust_view_converter::throwLastError("Rust M3C2 stage failed.");

    rust_view_converter::fromRust(rustOut, *m_p->cores);
    pdal_point_view_destroy(rustOut);
}

} // namespace pdal
