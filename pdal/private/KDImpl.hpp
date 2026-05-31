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
 *       the documentation and/or other materials provided with the
 *       distribution.
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

#pragma once

#include <pdal/private/RustViewConverter.hpp>

namespace pdal
{
namespace
{

inline void appendIds(const std::vector<pdal_spatial_result_t>& results,
                      PointIdList& output)
{
    output.reserve(results.size());
    for (const pdal_spatial_result_t& result : results)
        output.push_back(result.id);
}

inline void copyKnnResults(const std::vector<pdal_spatial_result_t>& results,
                           PointIdList* indices, std::vector<double>* sqrDists)
{
    if (!indices || !sqrDists)
        return;

    const size_t count =
        (std::min)({results.size(), indices->size(), sqrDists->size()});
    for (size_t i = 0; i < count; ++i)
    {
        (*indices)[i] = results[i].id;
        (*sqrDists)[i] = results[i].sqr_dist;
    }
}

inline std::vector<pdal_spatial_result_t>
knn(pdal_point_view_t* view, const std::vector<const char*>& dims,
    const std::vector<double>& query, point_count_t k, size_t stride)
{
    if (!view || dims.empty() || query.empty() || k == 0)
        return {};

    std::vector<pdal_spatial_result_t> results(k);
    uint64_t written = pdal_point_view_knn(
        view, dims.data(), query.data(), dims.size(), k,
        (std::max)(stride, size_t(1)), results.data(), results.size());
    results.resize(written);
    return results;
}

inline std::vector<pdal_spatial_result_t>
radius(pdal_point_view_t* view, const std::vector<const char*>& dims,
       const std::vector<double>& query, double r)
{
    if (!view || dims.empty() || query.empty())
        return {};

    uint64_t len = 0;
    pdal_spatial_result_t* raw = pdal_point_view_radius(
        view, dims.data(), query.data(), dims.size(), r, &len);
    if (!raw)
        return {};

    std::vector<pdal_spatial_result_t> results(raw, raw + len);
    pdal_spatial_results_free(raw, len);
    return results;
}

inline std::vector<const char*>
dimensionNames(const PointLayout& layout, const Dimension::IdList& dims,
               std::vector<std::string>& storage)
{
    storage.clear();
    storage.reserve(dims.size());
    std::vector<const char*> names;
    names.reserve(dims.size());
    for (Dimension::Id dim : dims)
    {
        storage.push_back(layout.dimName(dim));
        names.push_back(storage.back().c_str());
    }
    return names;
}

} // unnamed namespace

class KDBaseImpl
{
public:
    explicit KDBaseImpl(const PointView& buf) : m_buf(buf), m_rustView(nullptr)
    {
    }

    ~KDBaseImpl()
    {
        pdal_point_view_destroy(m_rustView);
    }

    void build()
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = rust_view_converter::toRust(const_cast<PointView&>(m_buf));
    }

protected:
    pdal_point_view_t* view() const
    {
        if (!m_rustView)
            const_cast<KDBaseImpl*>(this)->build();
        return m_rustView;
    }

private:
    const PointView& m_buf;
    pdal_point_view_t* m_rustView;
};

class KD2Impl : public KDBaseImpl
{
public:
    using RadiusResults = std::vector<std::pair<size_t, double>>;

    explicit KD2Impl(const PointView& buf) : KDBaseImpl(buf) {}

    void build()
    {
        KDBaseImpl::build();
    }

    PointIdList neighbors(double x, double y, point_count_t k) const
    {
        PointIdList output;
        appendIds(knn(view(), {"X", "Y"}, {x, y}, k, 1), output);
        return output;
    }

    void knnSearch(double x, double y, point_count_t k, PointIdList* indices,
                   std::vector<double>* sqrDists) const
    {
        copyKnnResults(knn(view(), {"X", "Y"}, {x, y}, k, 1), indices,
                       sqrDists);
    }

    PointIdList radius(double x, double y, double r) const
    {
        PointIdList output;
        appendIds(radiusResults(x, y, r), output);
        return output;
    }

    void radius(double x, double y, double r, RadiusResults& results) const
    {
        results.clear();
        for (const pdal_spatial_result_t& result : radiusResults(x, y, r))
            results.push_back({result.id, result.sqr_dist});
    }

private:
    std::vector<pdal_spatial_result_t> radiusResults(double x, double y,
                                                     double r) const
    {
        return pdal::radius(view(), {"X", "Y"}, {x, y}, r);
    }
};

class KD3Impl : public KDBaseImpl
{
public:
    using RadiusResults = std::vector<std::pair<size_t, double>>;

    explicit KD3Impl(const PointView& buf) : KDBaseImpl(buf) {}

    void build()
    {
        KDBaseImpl::build();
    }

    PointIdList neighbors(double x, double y, double z, point_count_t k,
                          size_t stride) const
    {
        PointIdList output;
        appendIds(knn(view(), {"X", "Y", "Z"}, {x, y, z}, k, stride), output);
        return output;
    }

    void knnSearch(double x, double y, double z, point_count_t k,
                   PointIdList* indices, std::vector<double>* sqrDists) const
    {
        copyKnnResults(knn(view(), {"X", "Y", "Z"}, {x, y, z}, k, 1), indices,
                       sqrDists);
    }

    PointIdList radius(double x, double y, double z, double r) const
    {
        PointIdList output;
        appendIds(radiusResults(x, y, z, r), output);
        return output;
    }

    void radius(double x, double y, double z, double r,
                RadiusResults& results) const
    {
        results.clear();
        for (const pdal_spatial_result_t& result : radiusResults(x, y, z, r))
            results.push_back({result.id, result.sqr_dist});
    }

private:
    std::vector<pdal_spatial_result_t> radiusResults(double x, double y,
                                                     double z, double r) const
    {
        return pdal::radius(view(), {"X", "Y", "Z"}, {x, y, z}, r);
    }
};

class KDFlexImpl : public KDBaseImpl
{
public:
    KDFlexImpl(const PointView& buf, const Dimension::IdList& dims)
        : KDBaseImpl(buf), m_buf(buf), m_dims(dims)
    {
    }

    void build()
    {
        KDBaseImpl::build();
    }

    PointIdList neighbors(PointRef& point, point_count_t k, size_t stride) const
    {
        std::vector<std::string> nameStorage;
        std::vector<const char*> names =
            dimensionNames(*m_buf.layout(), m_dims, nameStorage);
        PointIdList output;
        appendIds(knn(view(), names, query(point), k, stride), output);
        return output;
    }

    PointIdList radius(PointId idx, double r) const
    {
        std::vector<std::string> nameStorage;
        std::vector<const char*> names =
            dimensionNames(*m_buf.layout(), m_dims, nameStorage);
        PointIdList output;
        appendIds(pdal::radius(view(), names, query(idx), r), output);
        return output;
    }

private:
    std::vector<double> query(PointRef& point) const
    {
        std::vector<double> values;
        values.reserve(m_dims.size());
        for (Dimension::Id dim : m_dims)
            values.push_back(point.getFieldAs<double>(dim));
        return values;
    }

    std::vector<double> query(PointId idx) const
    {
        std::vector<double> values;
        values.reserve(m_dims.size());
        for (Dimension::Id dim : m_dims)
            values.push_back(m_buf.getFieldAs<double>(dim, idx));
        return values;
    }

    const PointView& m_buf;
    const Dimension::IdList& m_dims;
};

} // namespace pdal
