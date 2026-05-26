/******************************************************************************
 * Copyright (c) 2019, Helix Re Inc. nicolas@helix.re
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
 *     * Neither the name of Helix Re Inc. nor the
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

// This is an implementation of the local feature descriptors introduced in
// WEAKLY SUPERVISED SEGMENTATION-AIDED CLASSIFICATION OF URBANSCENES FROM 3D
// LIDAR POINT CLOUDS Stéphane Guinard, Loïc Landrieu, 2017

#include "CovarianceFeaturesFilter.hpp"

#include <pdal/private/RustViewConverter.hpp>
#include <pdal/util/ProgramArgs.hpp>

#include <pdal_capi.h>

#include <algorithm>
#include <string>
#include <vector>

namespace pdal
{
using namespace Dimension;

static StaticPluginInfo const s_info{
    "filters.covariancefeatures",
    "Filter that calculates local features based on the covariance matrix of a "
    "point's neighborhood.",
    "https://pdal.org/stages/filters.covariancefeatures.html"};

CREATE_STATIC_STAGE(CovarianceFeaturesFilter, s_info)

std::string CovarianceFeaturesFilter::getName() const
{
    return s_info.name;
}

std::istream& operator>>(std::istream& in, CovarianceFeaturesFilter::Mode& mode)
{
    std::string s;
    in >> s;

    s = Utils::tolower(s);
    if (s == "raw")
        mode = CovarianceFeaturesFilter::Mode::Raw;
    else if (s == "sqrt")
        mode = CovarianceFeaturesFilter::Mode::SQRT;
    else if (s == "normalized")
        mode = CovarianceFeaturesFilter::Mode::Normalized;
    else
        in.setstate(std::ios_base::failbit);
    return in;
}

std::ostream& operator<<(std::ostream& out,
                         const CovarianceFeaturesFilter::Mode& mode)
{
    switch (mode)
    {
    case CovarianceFeaturesFilter::Mode::Raw:
        out << "raw";
        break;
    case CovarianceFeaturesFilter::Mode::SQRT:
        out << "sqrt";
        break;
    case CovarianceFeaturesFilter::Mode::Normalized:
        out << "normalized";
        break;
    }
    return out;
}

void CovarianceFeaturesFilter::addArgs(ProgramArgs& args)
{
    args.add("knn", "k-Nearest neighbors", m_knn, 10);
    args.add("threads", "Number of threads used to run this filter", m_threads,
             1);
    args.add("feature_set", "Set of features to be computed",
             m_featureSetString, {"dimensionality"});
    args.add("stride", "Compute features on strided neighbors", m_stride,
             size_t(1));
    m_radiusArg =
        &args.add("radius", "Radius for nearest neighbor search", m_radius);
    args.add("min_k", "Minimum number of neighbors in radius", m_minK, 3);
    args.add("mode", "Raw, normalized, or sqrt of eigenvalues", m_mode,
             Mode::SQRT);
    args.add("optimized", "Use OptimalKNN or OptimalRadius?", m_optimal, false);
}

void CovarianceFeaturesFilter::addDimensions(PointLayoutPtr layout)
{
    for (auto& feat : m_featureSetString)
    {
        std::string featureSet = Utils::tolower(feat);
        Utils::trim(featureSet);
        if (featureSet == "dimensionality")
            m_extraDims.insert(m_extraDims.end(),
                               {Id::Linearity, Id::Planarity, Id::Scattering,
                                Id::Verticality});
        else if (featureSet == "all")
            m_extraDims.insert(m_extraDims.end(),
                               {Id::Linearity, Id::Planarity, Id::Scattering,
                                Id::Verticality, Id::Omnivariance,
                                Id::Anisotropy, Id::Eigenentropy,
                                Id::EigenvalueSum, Id::SurfaceVariation,
                                Id::DemantkeVerticality, Id::Density});
        else if (featureSet == "linearity")
            m_extraDims.push_back(Id::Linearity);
        else if (featureSet == "planarity")
            m_extraDims.push_back(Id::Planarity);
        else if (featureSet == "scattering")
            m_extraDims.push_back(Id::Scattering);
        else if (featureSet == "verticality")
            m_extraDims.push_back(Id::Verticality);
        else if (featureSet == "omnivariance")
            m_extraDims.push_back(Id::Omnivariance);
        else if (featureSet == "anisotropy")
            m_extraDims.push_back(Id::Anisotropy);
        else if (featureSet == "eigenentropy")
            m_extraDims.push_back(Id::Eigenentropy);
        else if (featureSet == "eigenvaluesum")
            m_extraDims.push_back(Id::EigenvalueSum);
        else if (featureSet == "surfacevariation")
            m_extraDims.push_back(Id::SurfaceVariation);
        else if (featureSet == "demantkeverticality")
            m_extraDims.push_back(Id::DemantkeVerticality);
        else if (featureSet == "density")
            m_extraDims.push_back(Id::Density);
    }

    layout->registerDims(m_extraDims);
}

void CovarianceFeaturesFilter::prepared(PointTableRef table)
{
    const PointLayoutPtr layout(table.layout());
    if (std::count(m_extraDims.begin(), m_extraDims.end(), Id::Density))
    {
        if (!(layout->hasDim(Id::OptimalKNN) &&
              layout->hasDim(Id::OptimalRadius)))
            throwError("Density feature requires OptimalKNN and OptimalRadius "
                       "dimensions, which are missing in the input PointView.");
    }
    if (m_optimal)
    {
        if (!layout->hasDim(Id::OptimalKNN))
            throwError("Missing OptimalKNN dimension in input PointView.");
    }
}

void CovarianceFeaturesFilter::filter(PointView& view)
{
    std::vector<const char*> dims;
    for (auto const& feat : m_featureSetString)
        dims.push_back(feat.c_str());

    pdal_stage_t* stage = pdal_stage_create_covariancefeatures(
        m_knn, m_radiusArg->set(), m_radius, m_minK, m_stride,
        static_cast<uint8_t>(m_mode), m_optimal, dims.data(), dims.size());
    if (!stage)
        rust_view_converter::throwLastError(
            "Failed to create Rust covariancefeatures stage.");

    rust_view_converter::runInPlace(stage, view);
    pdal_stage_destroy(stage);
}

} // namespace pdal
