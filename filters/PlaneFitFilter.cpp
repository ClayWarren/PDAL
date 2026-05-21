/******************************************************************************
 * Copyright (c) 2019, Bradley J Chambers (brad.chambers@gmail.com)
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

// PDAL implementation of the plane fit criterion presented in T. Weyrich, M.
// Pauly, R. Keiser, S. Heinzle, S. Scandella, and M. Gross, “Post-processing
// of Scanned 3D Surface Data,” Proc. Eurographics Symp.  Point-Based Graph.
// 2004, pp. 85–94, 2004.

#include "PlaneFitFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>

#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>

#include <string>

namespace pdal
{

using namespace Dimension;

static StaticPluginInfo const s_info{
    "filters.planefit", "Plane Fit (Kutz et al., 2003)",
    "https://pdal.org/stages/filters.planefit.html"};

CREATE_STATIC_STAGE(PlaneFitFilter, s_info)

std::string PlaneFitFilter::getName() const
{
    return s_info.name;
}

void PlaneFitFilter::addArgs(ProgramArgs& args)
{
    args.add("knn", "k-Nearest neighbors", m_knn, 8);
    args.add("threads", "Number of threads used to run this filter", m_threads,
             1);
}

void PlaneFitFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Id::PlaneFit);
}

void PlaneFitFilter::filter(PointView& view)
{
    pdal_stage_t* stage = pdal_stage_create_planefit(m_knn);
    if (!stage)
        throwError("Failed to create Rust plane fit stage.");

    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
