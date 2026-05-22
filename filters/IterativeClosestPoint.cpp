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

#include "IterativeClosestPoint.hpp"

#include <pdal/private/RustViewConverter.hpp>
#include <pdal/util/ProgramArgs.hpp>

#include <pdal_capi.h>

#include <Eigen/Dense>

#include <sstream>

namespace pdal
{

using namespace Dimension;
using namespace Eigen;

static StaticPluginInfo const s_info{
    "filters.icp", "Iterative Closest Point (ICP) registration.",
    "https://pdal.org/stages/filters.icp.html"};

CREATE_STATIC_STAGE(IterativeClosestPoint, s_info)

std::string IterativeClosestPoint::getName() const
{
    return s_info.name;
}

void IterativeClosestPoint::addArgs(ProgramArgs& args)
{
    args.add("max_iter", "Maximum number of iterations", m_max_iters, 100);
    args.add("rt", "Rotation threshold", m_rotation_threshold,
             0.99999); // 0.256 degrees
    args.add("tt", "Translation threshold", m_translation_threshold,
             3e-4 * 3e-4); // 0.0003 meters
    args.add("mse_abs", "Absolute threshold for MSE", m_mse_abs, 1e-12);
    args.add("max_similar",
             "Max number of similar transforms to consider converged",
             m_max_similar, 0);
    m_maxdistArg =
        &args.add("max_dist", "Maximum correspondence distance", m_maxdist);
    m_matrixArg =
        &args.add("init", "Initial transformation matrix", m_matrixStr);
}

void IterativeClosestPoint::prepared(PointTableRef table)
{
    if (m_matrixArg->set())
    {
        std::stringstream matrix;
        matrix.str(m_matrixStr);
        matrix.seekg(0);
        double val;
        while (matrix >> val)
            m_vec.push_back(val);
        if (m_vec.size() != 16)
            throwError("Expecting exactly 16 values in 'init' got " +
                       std::to_string(m_vec.size()));
    }
}

PointViewSet IterativeClosestPoint::run(PointViewPtr view)
{
    PointViewSet viewSet;
    if (this->m_fixed)
    {
        log()->get(LogLevel::Debug2) << "Calculating ICP\n";
        PointViewPtr result = this->icp(this->m_fixed, view);
        viewSet.insert(result);
        log()->get(LogLevel::Debug2) << "ICP complete\n";
        this->m_complete = true;
    }
    else
    {
        log()->get(LogLevel::Debug2) << "Adding fixed points\n";
        this->m_fixed = view;
    }
    return viewSet;
}

void IterativeClosestPoint::done(PointTableRef _)
{
    if (!this->m_complete)
    {
        throw pdal_error(
            "filters.icp must have two point view inputs, no more, no less");
    }
}

PointViewPtr IterativeClosestPoint::icp(PointViewPtr fixed,
                                        PointViewPtr moving) const
{
    // The ICP registration core runs through the Rust C ABI. C++ retains the
    // multi-view orchestration (run/done) and metadata reporting.
    pdal_point_view_t* rustFixed = rust_view_converter::toRust(fixed);
    pdal_point_view_t* rustMoving = rust_view_converter::toRust(moving);

    double transform[16] = {0.0};
    double centroidVals[3] = {0.0};
    bool converged = false;
    double mse = 0.0;
    const double* init = m_matrixArg->set() ? m_vec.data() : nullptr;

    pdal_point_view_t* rustResult = pdal_icp_register(
        rustFixed, rustMoving, m_max_iters, m_max_similar, m_rotation_threshold,
        m_translation_threshold, m_mse_abs, m_maxdistArg->set(), m_maxdist,
        m_matrixArg->set(), init, transform, centroidVals, &converged, &mse);

    pdal_point_view_destroy(rustFixed);
    pdal_point_view_destroy(rustMoving);

    if (!rustResult)
        throw pdal_error("filters.icp: Rust registration failed.");

    PointViewPtr result = rust_view_converter::fromRust(rustResult, moving);
    pdal_point_view_destroy(rustResult);

    // The final transformation is returned in row-major order.
    Matrix4d final_transformation;
    for (int r = 0; r < 4; ++r)
        for (int c = 0; c < 4; ++c)
            final_transformation(r, c) = transform[r * 4 + c];

    Vector3d centroid(centroidVals[0], centroidVals[1], centroidVals[2]);

    // Transformation to demean coords.
    Matrix4d pretrans = Matrix4d::Identity();
    pretrans.block<3, 1>(0, 3) = -centroid;

    // Transformation to return to global coords.
    Matrix4d posttrans = Matrix4d::Identity();
    posttrans.block<3, 1>(0, 3) = centroid;

    // The composed transformation is built from right to left in order of
    // operations.
    Matrix4d composed_transformation =
        posttrans * final_transformation * pretrans;

    // Populate metadata nodes to capture the final transformation, convergence
    // status, and MSE.
    Eigen::IOFormat MetadataFmt(Eigen::FullPrecision, Eigen::DontAlignCols, " ",
                                "\n", "", "", "", "");
    MetadataNode root = getMetadata();
    std::stringstream ss;
    ss << final_transformation.format(MetadataFmt);
    root.add("transform", ss.str());
    ss.str("");
    ss << composed_transformation.format(MetadataFmt);
    root.add("composed", ss.str());
    ss.str("");
    ss << centroid.format(MetadataFmt);
    root.add("centroid", ss.str());
    root.add("converged", converged);
    root.add("fitness", mse);

    return result;
}

} // namespace pdal
