/******************************************************************************
 * Copyright (c) 2016, 2017, 2019, 2020 Bradley J Chambers
 *(brad.chambers@gmail.com)
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

// Normal refinement algorithm is presented in [1] and adapted to PDAL based on
// the implementation provided in [2].
//
// [1] H. Hoppe, T.  DeRose, T. Duchamp, J. McDonald, and W. Stuetzle, "Surface
// reconstruction from unorganized points," Computer Graphics, vol. 26. no. 2,
// pp. 71-78, 1992.
// [2] https://github.com/CloudCompare/CloudCompare.

#include "NormalFilter.hpp"
#include "private/Point.hpp"

#include <pdal/private/RustViewConverter.hpp>
#include <pdal/util/ProgramArgs.hpp>

#include <pdal_capi.h>

#include <string>

namespace pdal
{

using namespace Dimension;

static StaticPluginInfo const s_info{
    "filters.normal", "Normal Filter",
    "https://pdal.org/stages/filters.normal.html"};

CREATE_STATIC_STAGE(NormalFilter, s_info)

struct NormalArgs
{
    int m_knn;
    filter::Point m_viewpoint;
    double m_radius;
    bool m_up;
    bool m_refine;
};

NormalFilter::NormalFilter()
    : m_args(new NormalArgs), m_viewpointArg(nullptr), m_radiusArg(nullptr),
      m_knnArg(nullptr)
{
}

NormalFilter::~NormalFilter() {}

std::string NormalFilter::getName() const
{
    return s_info.name;
}

void NormalFilter::addArgs(ProgramArgs& args)
{
    m_knnArg = &args.add("knn", "k-Nearest Neighbors", m_args->m_knn, 8);
    m_radiusArg = &args.add("radius", "Radius to use for neighbor search",
                            m_args->m_radius);
    m_viewpointArg = &args.add("viewpoint", "Viewpoint as WKT or GeoJSON",
                               m_args->m_viewpoint);
    args.add("always_up", "Normals always oriented with positive Z?",
             m_args->m_up, true);
    args.add("refine",
             "Refine normals using minimum spanning tree propagation?",
             m_args->m_refine, false);
}

void NormalFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDims(
        {Id::NormalX, Id::NormalY, Id::NormalZ, Id::Curvature});
}

// public method to access filter, used by GreedyProjection and Poisson filters
void NormalFilter::doFilter(PointView& view, int knn)
{
    m_args->m_knn = knn;
    ProgramArgs args;
    addArgs(args);
    // We're never parsing anything, so we'll just end up with default vals.
    // This makes sure that the arg pointer (m_viewpointArg) is valid.
    filter(view);
}

void NormalFilter::prepared(PointTableRef table)
{
    if (m_args->m_up && m_viewpointArg->set())
    {
        log()->get(LogLevel::Warning)
            << "Viewpoint provided. Ignoring always_up = TRUE." << '\n';
        m_args->m_up = false;
    }

    if (m_radiusArg->set())
    {
        if (m_knnArg->set())
            throwError("Cannot set both knn and radius.");
        m_args->m_knn = 0;
    }
    else
    {
        // The query point is returned as a neighbor of itself, so we must
        // increase k by one to get the desired number of neighbors.
        ++m_args->m_knn;
    }
}

void NormalFilter::filter(PointView& view)
{
    // Compute the normal/curvature and optional viewpoint/up orientation
    // through the Rust C ABI.
    bool hasViewpoint = m_viewpointArg->set();
    double vx = 0.0, vy = 0.0, vz = 0.0;
    if (hasViewpoint)
    {
        vx = m_args->m_viewpoint.x();
        vy = m_args->m_viewpoint.y();
        vz = m_args->m_viewpoint.z();
    }

    pdal_stage_t* stage = pdal_stage_create_normal(
        m_args->m_knn, m_radiusArg->set(), m_args->m_radius, hasViewpoint, vx,
        vy, vz, m_args->m_up, m_args->m_refine);
    if (!stage)
        throwError("Failed to create Rust normal stage.");

    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
