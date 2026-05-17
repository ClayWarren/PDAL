/******************************************************************************
 * Copyright (c) 2020, Bradley J Chambers (brad.chambers@gmail.com)
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

// PDAL implementation of W. Li, Q. Guo, M. K. Jakubowski, and M. Kelly, “A new
// method for segmenting individual trees from the lidar point cloud,”
// Photogramm. Eng. Remote Sensing, vol. 78, no. 1, pp. 75–84, 2012.

#include "LiTreeFilter.hpp"

#include "private/RustViewConverter.hpp"

#include <pdal_capi.h>

namespace pdal
{

using namespace Dimension;

static PluginInfo const s_info{"filters.litree", "Li Tree Filter",
                               "https://pdal.org/stages/filters.litree.html"};

CREATE_STATIC_STAGE(LiTreeFilter, s_info)

std::string LiTreeFilter::getName() const
{
    return s_info.name;
}

void LiTreeFilter::addArgs(ProgramArgs& args)
{
    args.add("min_points", "Minimum number of points in a tree cluster",
             m_minSize, point_count_t(10));
    args.add("min_height",
             "Minimum height above ground to start a tree cluster", m_minHag,
             3.0);
    args.add("radius", "Dummy point located outside this approximate radius",
             m_dummyRadius, 100.0);
}

void LiTreeFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Id::ClusterID);
}

void LiTreeFilter::prepared(PointTableRef table)
{
    const PointLayoutPtr layout(table.layout());
    if (!layout->hasDim(Id::HeightAboveGround))
        throwError("Missing HeightAboveGround dimension in input PointView.");
}

void LiTreeFilter::filter(PointView& view)
{
    pdal_stage_t* stage =
        pdal_stage_create_litree(m_minSize, m_minHag, m_dummyRadius);
    if (!stage)
        throwError("Failed to create Rust litree stage.");

    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
