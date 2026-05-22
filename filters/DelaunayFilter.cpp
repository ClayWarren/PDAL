/******************************************************************************
 * Copyright (c) 2018, Danish Agency for Data Supply and Efficiency,
 * sdfe@sdfe.dk
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
 *     * Neither the name of SDFE nor the names of its contributors may be
 *       used to endorse or promote products derived from this software
 *       without specific prior written permission.
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

#include "DelaunayFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>
#include <pdal_capi.h>

namespace pdal
{

static PluginInfo const s_info{"filters.delaunay",
                               "Perform Delaunay triangulation of a pointcloud",
                               "https://pdal.org/stages/filters.delaunay.html"};

CREATE_STATIC_STAGE(DelaunayFilter, s_info)

std::string DelaunayFilter::getName() const
{
    return s_info.name;
}

DelaunayFilter::DelaunayFilter() {}

DelaunayFilter::~DelaunayFilter() {}

void DelaunayFilter::filter(PointView& pointView)
{
    // Returns NULL if the mesh already exists
    TriangularMesh* mesh = pointView.createMesh("delaunay2d");

    if (!mesh)
        throwError("Unable to create mesh 'delaunay2d'.");
    if (pointView.size() < 3)
    {
        log()->get(LogLevel::Warning)
            << getName()
            << ": triangulation "
               "requested for fewer than three points.\n";
        return;
    }

    // Perform the triangulation through the Rust C ABI.
    pdal_point_view_t* rustIn = rust_view_converter::toRust(pointView);
    uint64_t triLen = 0;
    uint64_t* triangles = pdal_delaunay_triangulate(rustIn, &triLen);
    pdal_point_view_destroy(rustIn);

    if (triangles)
    {
        for (uint64_t i = 0; i + 2 < triLen; i += 3)
            mesh->add(triangles[i + 2], triangles[i + 1], triangles[i]);
        pdal_free_u64_array(triangles, triLen);
    }
}

} // namespace pdal
