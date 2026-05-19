/******************************************************************************
 * Copyright (c) 2020, Hobu Inc.
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

#include "KDIndex.hpp"
#include "private/KDImpl.hpp"
#include <rust/pdal-capi/include/pdal_capi.h>

namespace pdal
{

namespace
{

int typeId(Dimension::Type type)
{
    using Dimension::Type;
    switch (type)
    {
    case Type::Unsigned8:
        return 0;
    case Type::Unsigned16:
        return 1;
    case Type::Unsigned32:
        return 2;
    case Type::Unsigned64:
        return 3;
    case Type::Signed8:
        return 4;
    case Type::Signed16:
        return 5;
    case Type::Signed32:
        return 6;
    case Type::Signed64:
        return 7;
    case Type::Float:
        return 8;
    case Type::Double:
    case Type::None:
        return 9;
    }
    return 9;
}

pdal_point_view_t* toRustPointView(const PointView& view)
{
    pdal_point_layout_t* layout = pdal_point_layout_create();
    for (auto dim : view.layout()->dims())
    {
        pdal_point_layout_register_dim(layout,
                                       view.layout()->dimName(dim).c_str(),
                                       typeId(view.layout()->dimType(dim)));
    }

    pdal_point_view_t* rustView = pdal_point_view_create(layout);
    for (PointId idx = 0; idx < view.size(); ++idx)
    {
        pdal_point_view_add_point(rustView);
        for (auto dim : view.layout()->dims())
        {
            const std::string name = view.layout()->dimName(dim);
            pdal_point_view_set_f64(rustView, idx, name.c_str(),
                                    view.getFieldAs<double>(dim, idx));
        }
    }
    return rustView;
}

PointIdList knn(const PointView& view, StringList dimNames,
                std::vector<double> query, point_count_t k, size_t stride,
                std::vector<double>* sqrDists = nullptr)
{
    k = (std::min)(view.size(), k);
    PointIdList ids(k);
    std::vector<pdal_spatial_result_t> results(k);
    std::vector<const char*> dimPtrs;
    for (const std::string& name : dimNames)
        dimPtrs.push_back(name.c_str());

    pdal_point_view_t* rustView = toRustPointView(view);
    uint64_t count = pdal_point_view_knn(rustView, dimPtrs.data(), query.data(),
                                         dimPtrs.size(), k, stride,
                                         results.data(), results.size());
    pdal_point_view_destroy(rustView);

    ids.resize(count);
    if (sqrDists)
        sqrDists->resize(count);
    for (uint64_t i = 0; i < count; ++i)
    {
        ids[i] = results[i].id;
        if (sqrDists)
            (*sqrDists)[i] = results[i].sqr_dist;
    }
    return ids;
}

PointIdList
rustRadius(const PointView& view, StringList dimNames,
           std::vector<double> query, double r,
           std::vector<std::pair<size_t, double>>* resultsOut = nullptr)
{
    std::vector<const char*> dimPtrs;
    for (const std::string& name : dimNames)
        dimPtrs.push_back(name.c_str());

    pdal_point_view_t* rustView = toRustPointView(view);
    uint64_t count = 0;
    pdal_spatial_result_t* results = pdal_point_view_radius(
        rustView, dimPtrs.data(), query.data(), dimPtrs.size(), r, &count);
    pdal_point_view_destroy(rustView);

    PointIdList ids(count);
    if (resultsOut)
        resultsOut->resize(count);
    for (uint64_t i = 0; i < count; ++i)
    {
        ids[i] = results[i].id;
        if (resultsOut)
            (*resultsOut)[i] =
                std::make_pair(results[i].id, results[i].sqr_dist);
    }
    pdal_spatial_results_free(results, count);
    return ids;
}

} // unnamed namespace

//
// KD2Index
//

KD2Index::KD2Index(const PointView& buf)
    : m_buf(buf), m_impl(new KD2Impl(m_buf))
{
    if (!m_buf.hasDim(Dimension::Id::X))
        throw pdal_error("KD2Index: point view missing 'X' dimension.");
    if (!m_buf.hasDim(Dimension::Id::Y))
        throw pdal_error("KD2Index: point view missing 'Y' dimension.");
}

KD2Index::~KD2Index() {}

void KD2Index::build()
{
    m_impl->build();
}

PointId KD2Index::neighbor(double x, double y) const
{
    PointIdList ids = neighbors(x, y, 1);
    return (ids.size() ? ids[0] : 0);
}

PointId KD2Index::neighbor(PointId idx) const
{
    PointIdList ids = neighbors(idx, 1);
    return (ids.size() ? ids[0] : 0);
}

PointId KD2Index::neighbor(PointRef& point) const
{
    PointIdList ids = neighbors(point, 1);
    return (ids.size() ? ids[0] : 0);
}

PointIdList KD2Index::neighbors(double x, double y, point_count_t k) const
{
    return knn(m_buf, {"X", "Y"}, {x, y}, k, 1);
}

PointIdList KD2Index::neighbors(PointId idx, point_count_t k) const
{
    double x = m_buf.getFieldAs<double>(Dimension::Id::X, idx);
    double y = m_buf.getFieldAs<double>(Dimension::Id::Y, idx);

    return neighbors(x, y, k);
}

PointIdList KD2Index::neighbors(PointRef& point, point_count_t k) const
{
    double x = point.getFieldAs<double>(Dimension::Id::X);
    double y = point.getFieldAs<double>(Dimension::Id::Y);

    return neighbors(x, y, k);
}

void KD2Index::knnSearch(double x, double y, point_count_t k,
                         PointIdList* indices,
                         std::vector<double>* sqr_dists) const
{
    *indices = knn(m_buf, {"X", "Y"}, {x, y}, k, 1, sqr_dists);
}

void KD2Index::knnSearch(PointId idx, point_count_t k, PointIdList* indices,
                         std::vector<double>* sqr_dists) const
{
    double x = m_buf.getFieldAs<double>(Dimension::Id::X, idx);
    double y = m_buf.getFieldAs<double>(Dimension::Id::Y, idx);

    knnSearch(x, y, k, indices, sqr_dists);
}

void KD2Index::knnSearch(PointRef& point, point_count_t k, PointIdList* indices,
                         std::vector<double>* sqr_dists) const
{
    double x = point.getFieldAs<double>(Dimension::Id::X);
    double y = point.getFieldAs<double>(Dimension::Id::Y);

    knnSearch(x, y, k, indices, sqr_dists);
}

PointIdList KD2Index::radius(double x, double y, double r) const
{
    return rustRadius(m_buf, {"X", "Y"}, {x, y}, r);
}

void KD2Index::radius(double x, double y, double r,
                      KD2Index::RadiusResults& result) const
{
    rustRadius(m_buf, {"X", "Y"}, {x, y}, r, &result);
}

void KD2Index::radius(PointId idx, double r,
                      KD2Index::RadiusResults& result) const
{
    double x = m_buf.getFieldAs<double>(Dimension::Id::X, idx);
    double y = m_buf.getFieldAs<double>(Dimension::Id::Y, idx);

    return radius(x, y, r, result);
}

PointIdList KD2Index::radius(PointId idx, double r) const
{
    double x = m_buf.getFieldAs<double>(Dimension::Id::X, idx);
    double y = m_buf.getFieldAs<double>(Dimension::Id::Y, idx);

    return radius(x, y, r);
}

PointIdList KD2Index::radius(PointRef& point, double r) const
{
    double x = point.getFieldAs<double>(Dimension::Id::X);
    double y = point.getFieldAs<double>(Dimension::Id::Y);

    return radius(x, y, r);
}

//
// KD3Index
//

KD3Index::KD3Index(const PointView& buf)
    : m_buf(buf), m_impl(new KD3Impl(m_buf))
{
    if (!m_buf.hasDim(Dimension::Id::X))
        throw pdal_error("KD3Index: point view missing 'X' dimension.");
    if (!m_buf.hasDim(Dimension::Id::Y))
        throw pdal_error("KD3Index: point view missing 'Y' dimension.");
    if (!m_buf.hasDim(Dimension::Id::Z))
        throw pdal_error("KD3Index: point view missing 'Z' dimension.");
}

KD3Index::~KD3Index() {}

void KD3Index::build()
{
    m_impl->build();
}

PointId KD3Index::neighbor(double x, double y, double z) const
{
    PointIdList ids = neighbors(x, y, z, 1);
    return (ids.size() ? ids[0] : 0);
}

PointId KD3Index::neighbor(PointId idx) const
{
    PointIdList ids = neighbors(idx, 1);
    return (ids.size() ? ids[0] : 0);
}

PointId KD3Index::neighbor(PointRef& point) const
{
    PointIdList ids = neighbors(point, 1);
    return (ids.size() ? ids[0] : 0);
}

PointIdList KD3Index::neighbors(double x, double y, double z, point_count_t k,
                                size_t stride) const
{
    return knn(m_buf, {"X", "Y", "Z"}, {x, y, z}, k, stride);
}

PointIdList KD3Index::neighbors(PointId idx, point_count_t k,
                                size_t stride) const
{
    double x = m_buf.getFieldAs<double>(Dimension::Id::X, idx);
    double y = m_buf.getFieldAs<double>(Dimension::Id::Y, idx);
    double z = m_buf.getFieldAs<double>(Dimension::Id::Z, idx);

    return neighbors(x, y, z, k, stride);
}

PointIdList KD3Index::neighbors(PointRef& point, point_count_t k,
                                size_t stride) const
{
    double x = point.getFieldAs<double>(Dimension::Id::X);
    double y = point.getFieldAs<double>(Dimension::Id::Y);
    double z = point.getFieldAs<double>(Dimension::Id::Z);

    return neighbors(x, y, z, k, stride);
}

void KD3Index::knnSearch(double x, double y, double z, point_count_t k,
                         PointIdList* indices,
                         std::vector<double>* sqr_dists) const
{
    *indices = knn(m_buf, {"X", "Y", "Z"}, {x, y, z}, k, 1, sqr_dists);
}

void KD3Index::knnSearch(PointId idx, point_count_t k, PointIdList* indices,
                         std::vector<double>* sqr_dists) const
{
    double x = m_buf.getFieldAs<double>(Dimension::Id::X, idx);
    double y = m_buf.getFieldAs<double>(Dimension::Id::Y, idx);
    double z = m_buf.getFieldAs<double>(Dimension::Id::Z, idx);

    knnSearch(x, y, z, k, indices, sqr_dists);
}

void KD3Index::knnSearch(PointRef& point, point_count_t k, PointIdList* indices,
                         std::vector<double>* sqr_dists) const
{
    double x = point.getFieldAs<double>(Dimension::Id::X);
    double y = point.getFieldAs<double>(Dimension::Id::Y);
    double z = point.getFieldAs<double>(Dimension::Id::Z);

    knnSearch(x, y, z, k, indices, sqr_dists);
}

PointIdList KD3Index::radius(double x, double y, double z, double r) const
{
    return rustRadius(m_buf, {"X", "Y", "Z"}, {x, y, z}, r);
}

void KD3Index::radius(double x, double y, double z, double r,
                      KD3Index::RadiusResults& results) const
{
    rustRadius(m_buf, {"X", "Y", "Z"}, {x, y, z}, r, &results);
}

void KD3Index::radius(PointId idx, double r,
                      KD3Index::RadiusResults& results) const
{
    double x = m_buf.getFieldAs<double>(Dimension::Id::X, idx);
    double y = m_buf.getFieldAs<double>(Dimension::Id::Y, idx);
    double z = m_buf.getFieldAs<double>(Dimension::Id::Z, idx);

    radius(x, y, z, r, results);
}

PointIdList KD3Index::radius(PointId idx, double r) const
{
    double x = m_buf.getFieldAs<double>(Dimension::Id::X, idx);
    double y = m_buf.getFieldAs<double>(Dimension::Id::Y, idx);
    double z = m_buf.getFieldAs<double>(Dimension::Id::Z, idx);

    return radius(x, y, z, r);
}

PointIdList KD3Index::radius(PointRef& point, double r) const
{
    double x = point.getFieldAs<double>(Dimension::Id::X);
    double y = point.getFieldAs<double>(Dimension::Id::Y);
    double z = point.getFieldAs<double>(Dimension::Id::Z);

    return radius(x, y, z, r);
}

//
// KDFlexIndex
//

KDFlexIndex::KDFlexIndex(const PointView& buf, const Dimension::IdList& dims)
    : m_buf(buf), m_dims(dims), m_impl(new KDFlexImpl(m_buf, m_dims))
{
}

KDFlexIndex::~KDFlexIndex() {}

void KDFlexIndex::build()
{
    m_impl->build();
}

PointId KDFlexIndex::neighbor(PointRef& point) const
{
    PointIdList ids = neighbors(point, 1);
    return (ids.size() ? ids[0] : 0);
}

PointIdList KDFlexIndex::neighbors(PointRef& point, point_count_t k,
                                   size_t stride) const
{
    StringList dimNames;
    std::vector<double> query;
    for (auto dim : m_dims)
    {
        dimNames.push_back(m_buf.layout()->dimName(dim));
        query.push_back(point.getFieldAs<double>(dim));
    }
    return knn(m_buf, dimNames, query, k, stride);
}

PointIdList KDFlexIndex::radius(PointId idx, double r) const
{
    StringList dimNames;
    std::vector<double> query;
    for (auto dim : m_dims)
    {
        dimNames.push_back(m_buf.layout()->dimName(dim));
        query.push_back(m_buf.getFieldAs<double>(dim, idx));
    }
    return rustRadius(m_buf, dimNames, query, r);
}

} // namespace pdal
