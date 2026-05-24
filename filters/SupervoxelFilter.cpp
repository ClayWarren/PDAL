/******************************************************************************
 * Copyright (c) 2025, Bram Ton (bram@cbbg.nl)
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

// Yangbin Lin, Cheng Wang, Dawei Zhai, Wei Li, Jonathan Li,
// Toward better boundary preserved supervoxel segmentation for 3D point clouds,
// ISPRS Journal of Photogrammetry and Remote Sensing, Volume 143, 2018,
// Pages 39-47, ISSN 0924-2716, doi:10.1016/j.isprsjprs.2018.05.004
//
// This implementation is derived from the work of the original authors:
// https://github.com/yblin/Supervoxel-for-3D-point-clouds

#include "SupervoxelFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>
#include <pdal_capi.h>

#include <string>

namespace pdal
{

using namespace Dimension;

static StaticPluginInfo const s_info{
    "filters.supervoxel", "Supervoxel segmentation.",
    "https://pdal.org/stages/filters.supervoxel.html"};

CREATE_STATIC_STAGE(SupervoxelFilter, s_info)

std::string SupervoxelFilter::getName() const
{
    return s_info.name;
}

SupervoxelFilter::SupervoxelFilter() : Filter() {}

void SupervoxelFilter::addArgs(ProgramArgs& args)
{
    args.add("knn", "k nearest neighbours", m_knn, static_cast<uint64_t>(32));
    args.add("resolution", "Resolution", m_R, 1.0);
}

void SupervoxelFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Id::ClusterID);
}

void SupervoxelFilter::prepared(PointTableRef table)
{
    const PointLayoutPtr layout(table.layout());
    if (!(layout->hasDim(Id::NormalX)) || !(layout->hasDim(Id::NormalY)) ||
        !(layout->hasDim(Id::NormalZ)))
        throwError("No normals found.");
}

void SupervoxelFilter::filter(PointView& view)
{
    pdal_stage_t* stage = pdal_stage_create_supervoxel(m_knn, m_R);
    if (!stage)
        throwError("Failed to create Rust supervoxel stage.");

    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
