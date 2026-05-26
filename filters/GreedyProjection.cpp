/*
 * Software License Agreement (BSD License)
 *
 *  Copyright (c) 2010, Willow Garage, Inc.
 *  All rights reserved.
 *
 *  Redistribution and use in source and binary forms, with or without
 *  modification, are permitted provided that the following conditions
 *  are met:
 *
 *   * Redistributions of source code must retain the above copyright
 *     notice, this list of conditions and the following disclaimer.
 *   * Redistributions in binary form must reproduce the above
 *     copyright notice, this list of conditions and the following
 *     disclaimer in the documentation and/or other materials provided
 *     with the distribution.
 *   * Neither the name of Willow Garage, Inc. nor the names of its
 *     contributors may be used to endorse or promote products derived
 *     from this software without specific prior written permission.
 *
 *  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 *  "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 *  LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 *  FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 *  COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 *  INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 *  BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 *  LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 *  CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 *  LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
 *  ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 *  POSSIBILITY OF SUCH DAMAGE.
 *
 * $Id$
 *
 */

#include <cassert>

#include <filters/NormalFilter.hpp>
#include <pdal/KDIndex.hpp>
#include <pdal/private/RustViewConverter.hpp>
#include <pdal_capi.h>

#include "GreedyProjection.hpp"

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.greedyprojection", "Greedy Triangulation filter",
    "https://pdal.org/stages/filters.greedyprojection.html"};

CREATE_STATIC_STAGE(GreedyProjection, s_info)

std::string GreedyProjection::getName() const
{
    return s_info.name;
}

void GreedyProjection::addArgs(ProgramArgs& args)
{
    args.add("multiplier", "Nearest neighbor distance multiplier", mu_)
        .setPositional();
    args.add("radius", "Search radius for neighbors", search_radius_)
        .setPositional();
    args.add("num_neighbors", "Number of nearest neighbors to consider", nnn_,
             100);
    args.add("min_angle", "Minimum angle for created triangles", minimum_angle_,
             M_PI / 18); // 10 degrees default
    args.add("max_angle", "Maximum angle for created triangles", maximum_angle_,
             2 * M_PI / 3); // 120 degrees default
    args.add("eps_angle",
             "Max normal difference angle for triangulation "
             "consideration",
             eps_angle_, M_PI / 4);
}

void GreedyProjection::addDimensions(PointLayoutPtr layout)
{
    layout->registerDims({Dimension::Id::NormalX, Dimension::Id::NormalY,
                          Dimension::Id::NormalZ});
}

void GreedyProjection::initialize()
{
    if (pdal_filter_greedyprojection_validate_options(mu_, search_radius_) != 0)
    {
        const char* err = pdal_last_error();
        std::string message =
            err ? std::string(err) : std::string("Invalid greedyprojection options.");
        throwError(message);
    }
}

void GreedyProjection::filter(PointView& view)
{
    NormalFilter nf;
    nf.setLog(log());
    nf.doFilter(view);

    mesh_ = view.createMesh(getName());

    pdal_point_view_t* rustIn = rust_view_converter::toRust(view);
    uint64_t count = 0;
    uint64_t* triangles = pdal_greedyprojection_triangulate(
        rustIn, mu_, search_radius_, nnn_, minimum_angle_, maximum_angle_,
        eps_angle_, consistent_, &count);
    pdal_point_view_destroy(rustIn);

    if (triangles)
    {
        for (uint64_t i = 0; i < count; i += 3)
        {
            mesh_->add(triangles[i], triangles[i + 1], triangles[i + 2]);
        }
        pdal_free_u64_array(triangles, count);
    }
}

} // namespace pdal
