/******************************************************************************
 * Copyright (c) 2016, Bradley J Chambers (brad.chambers@gmail.com)
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

#include "EigenvaluesFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>

#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>

#include <string>

namespace pdal
{

using namespace Dimension;

static StaticPluginInfo const s_info{
    "filters.eigenvalues",
    "Returns the eigenvalues for a given point, based on its k-nearest "
    "neighbors.",
    "https://pdal.org/stages/filters.eigenvalues.html"};

CREATE_STATIC_STAGE(EigenvaluesFilter, s_info)

struct EigenvalueArgs
{
    int m_knn;
    bool m_normalize;
    size_t m_stride;
    double m_radius;
    Arg* m_radiusArg;
    int m_minK;
};

EigenvaluesFilter::EigenvaluesFilter() : m_args(new EigenvalueArgs) {}

std::string EigenvaluesFilter::getName() const
{
    return s_info.name;
}

void EigenvaluesFilter::addArgs(ProgramArgs& args)
{
    args.add("knn", "k-Nearest neighbors", m_args->m_knn, 8);
    args.add("normalize", "Normalize eigenvalues?", m_args->m_normalize, false);
    args.add("stride", "Compute features on strided neighbors",
             m_args->m_stride, size_t(1));
    m_args->m_radiusArg = &args.add(
        "radius", "Radius for nearest neighbor search", m_args->m_radius);
    args.add("min_k", "Minimum number of neighbors in radius", m_args->m_minK,
             3);
}

void EigenvaluesFilter::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(Id::Eigenvalue0);
    layout->registerDim(Id::Eigenvalue1);
    layout->registerDim(Id::Eigenvalue2);
}

void EigenvaluesFilter::prepared(PointTableRef table)
{
    if (m_args->m_radiusArg->set())
    {
        log()->get(LogLevel::Warning)
            << "Radius has been set. Ignoring knn and stride values." << '\n';
        if (m_args->m_radius <= 0.0)
            log()->get(LogLevel::Error)
                << "Radius must be greater than 0." << '\n';
    }
    else
    {
        log()->get(LogLevel::Warning) << "No radius specified. Proceeding with "
                                         "knn and stride, but ignoring min_k."
                                      << '\n';
    }
}

void EigenvaluesFilter::filter(PointView& view)
{
    pdal_stage_t* stage = pdal_stage_create_eigenvalues(
        m_args->m_knn, m_args->m_normalize, m_args->m_stride,
        m_args->m_radiusArg->set(), m_args->m_radius, m_args->m_minK);
    if (!stage)
        throwError("Failed to create Rust eigenvalues stage.");

    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
