/******************************************************************************
 * Copyright (c) 2024, Bradley J Chambers (brad.chambers@gmail.com)
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

#include "SparseSurfaceFilter.hpp"

#include "private/RustViewConverter.hpp"

#include <pdal_capi.h>

namespace pdal
{

using namespace Dimension;

static PluginInfo const s_info{
    "filters.sparsesurface", "Sparse Surface Filter",
    "https://pdal.org/stages/filters.sparsesurface.html"};

CREATE_STATIC_STAGE(SparseSurfaceFilter, s_info)

std::string SparseSurfaceFilter::getName() const
{
    return s_info.name;
}

void SparseSurfaceFilter::addArgs(ProgramArgs& args)
{
    args.add("radius", "Mask neighbor points as low noise", m_radius, 1.0);
    args.add("ground_class",
             "Classification value of ground points."
             " [Default: 2]",
             m_groundClass, ClassLabel::Ground);
    args.add("low_point_class",
             "Classification value of non-ground points."
             " [Default: 7]",
             m_lowPointClass, ClassLabel::LowPoint);
}

void SparseSurfaceFilter::prepared(PointTableRef table)
{
    if (m_groundClass == m_lowPointClass)
    {
        throwError("Ground and low point class cannot be equal.");
    }
}

void SparseSurfaceFilter::filter(PointView& view)
{
    pdal_stage_t* stage = pdal_stage_create_sparsesurface(
        m_radius, m_groundClass, m_lowPointClass);
    if (!stage)
        throwError("Failed to create Rust sparse surface stage.");

    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
