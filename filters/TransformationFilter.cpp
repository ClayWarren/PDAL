/******************************************************************************
 * Copyright (c) 2014, Pete Gadomski <pete.gadomski@gmail.com>
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

#include "TransformationFilter.hpp"
#include <pdal/private/RustViewConverter.hpp>
#include <pdal/util/FileUtils.hpp>
#include <pdal_capi.h>

#include <Eigen/Dense>

#include <sstream>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.transformation",
    "Transform each point using a 4x4 transformation matrix",
    "https://pdal.org/stages/filters.transformation.html"};

CREATE_STATIC_STAGE(TransformationFilter, s_info)

TransformationFilter::Transform::Transform() {}

TransformationFilter::Transform::Transform(
    const TransformationFilter::Transform::ArrayType& arr)
    : m_vals(arr)
{
}

std::istream& operator>>(std::istream& in,
                         pdal::TransformationFilter::Transform& xform)
{
    std::string arg(std::istreambuf_iterator<char>(in), {});

    std::string matrix_str(arg);
    if (pdal::FileUtils::fileExists(arg))
        matrix_str = pdal::FileUtils::readFileIntoString(arg);

    double matrix[TransformationFilter::Transform::Size]{};
    char* error = pdal_transformation_matrix_parse(matrix_str.c_str(), matrix);
    if (error)
    {
        std::string message(error);
        pdal_string_free(error);
        throw pdal_error("filters.transformation: " + message);
    }

    for (size_t i = 0; i < xform.Size; ++i)
        xform[i] = matrix[i];
    in.clear();

    return in;
}

std::ostream& operator<<(std::ostream& out,
                         const pdal::TransformationFilter::Transform& xform)
{
    double matrix[TransformationFilter::Transform::Size]{};
    for (size_t i = 0; i < xform.Size; ++i)
        matrix[i] = xform[i];

    char* formatted = pdal_transformation_matrix_format(matrix);
    out << formatted;
    pdal_string_free(formatted);
    return out;
}

TransformationFilter::TransformationFilter()
    : m_matrix(new Transform), m_rust_stage(nullptr)
{
}

TransformationFilter::~TransformationFilter()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

std::string TransformationFilter::getName() const
{
    return s_info.name;
}

void TransformationFilter::addArgs(ProgramArgs& args)
{
    args.add("invert", "Apply inverse transformation", m_invert, false);
    args.add("matrix", "Transformation matrix", *m_matrix).setPositional();
    args.add("override_srs", "Spatial reference to apply to data.",
             m_overrideSrs);
}

void TransformationFilter::initialize()
{
    if (!m_overrideSrs.empty())
        setSpatialReference(m_overrideSrs);

    if (m_invert)
    {
        using namespace Eigen;

        Transform& matrix = *m_matrix;

        Affine3d T;
        Matrix4d m;
        m << matrix[0], matrix[1], matrix[2], matrix[3], matrix[4], matrix[5],
            matrix[6], matrix[7], matrix[8], matrix[9], matrix[10], matrix[11],
            matrix[12], matrix[13], matrix[14], matrix[15];
        T.matrix() = m;
        Affine3d Tinv = T.inverse();
        matrix[0] = Tinv.matrix()(0, 0);
        matrix[1] = Tinv.matrix()(0, 1);
        matrix[2] = Tinv.matrix()(0, 2);
        matrix[3] = Tinv.matrix()(0, 3);
        matrix[4] = Tinv.matrix()(1, 0);
        matrix[5] = Tinv.matrix()(1, 1);
        matrix[6] = Tinv.matrix()(1, 2);
        matrix[7] = Tinv.matrix()(1, 3);
        matrix[8] = Tinv.matrix()(2, 0);
        matrix[9] = Tinv.matrix()(2, 1);
        matrix[10] = Tinv.matrix()(2, 2);
        matrix[11] = Tinv.matrix()(2, 3);
    }
}

void TransformationFilter::prepared(PointTableRef table)
{
    (void)table;
    std::vector<double> mat_vals(16);
    for (size_t i = 0; i < 16; ++i)
        mat_vals[i] = (*m_matrix)[i];

    pdal_stage_t* stage = pdal_stage_create_transformation(mat_vals.data());
    if (!stage)
        rust_view_converter::throwLastError("Rust C ABI call failed.");
    pdal_stage_destroy(stage);
}

void TransformationFilter::doFilter(
    PointView& view, const TransformationFilter::Transform& matrix)
{
    *m_matrix = matrix;
    filter(view);
}

bool TransformationFilter::processOne(PointRef& point)
{
    Transform& matrix = *m_matrix;

    double x = point.getFieldAs<double>(Dimension::Id::X);
    double y = point.getFieldAs<double>(Dimension::Id::Y);
    double z = point.getFieldAs<double>(Dimension::Id::Z);
    double s = x * matrix[12] + y * matrix[13] + z * matrix[14] + matrix[15];

    point.setField(Dimension::Id::X,
                   (x * matrix[0] + y * matrix[1] + z * matrix[2] + matrix[3]) /
                       s);

    point.setField(Dimension::Id::Y,
                   (x * matrix[4] + y * matrix[5] + z * matrix[6] + matrix[7]) /
                       s);

    point.setField(
        Dimension::Id::Z,
        (x * matrix[8] + y * matrix[9] + z * matrix[10] + matrix[11]) / s);
    return true;
}

void TransformationFilter::spatialReferenceChanged(const SpatialReference& srs)
{
    if (!srs.empty() && !m_overrideSrs.empty())
        log()->get(LogLevel::Warning)
            << getName() << ": overriding input spatial reference." << '\n';
}

void TransformationFilter::filter(PointView& view)
{
    if (!view.spatialReference().empty() && !m_overrideSrs.empty())
        log()->get(LogLevel::Warning)
            << getName() << ": overriding input spatial reference." << '\n';

    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);

    std::vector<double> mat_vals(16);
    for (size_t i = 0; i < 16; ++i)
        mat_vals[i] = (*m_matrix)[i];

    m_rust_stage = pdal_stage_create_transformation(mat_vals.data());
    if (!m_rust_stage)
        throwError("Failed to create Rust transformation stage.");

    rust_view_converter::runInPlace(m_rust_stage, view);

    view.invalidateProducts();
}

} // namespace pdal
