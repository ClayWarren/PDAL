/******************************************************************************
 * Copyright (c) 2016-2017, Bradley J. Chambers (brad.chambers@gmail.com)
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

#pragma once

#include <pdal/Dimension.hpp>
#include <pdal/KDIndex.hpp>
#include <pdal/PointView.hpp>
#include <pdal/pdal_export.hpp>
#include <pdal/pdal_types.hpp>

#include <pdal_capi.h>

#include "DimRange.hpp"

#include <deque>
#include <type_traits>
#include <vector>

namespace pdal
{

class PointView;

namespace Segmentation
{

class PointClasses
{
    static const uint8_t Synthetic = 32;
    static const uint8_t Keypoint = 64;
    static const uint8_t Withheld = 128;

public:
    PointClasses() : m_classes(0) {}

    bool isWithheld() const
    {
        return m_classes & Withheld;
    }
    bool isKeypoint() const
    {
        return m_classes & Keypoint;
    }
    bool isSynthetic() const
    {
        return m_classes & Synthetic;
    }
    bool isNone() const
    {
        return m_classes == 0;
    }
    uint32_t bits() const
    {
        return m_classes;
    }

private:
    uint32_t m_classes;

    friend std::istream& operator>>(std::istream& in, PointClasses& classes);
    friend std::ostream& operator<<(std::ostream& out,
                                    const PointClasses& classes);
};

std::istream& operator>>(std::istream& in, PointClasses& classes);
std::ostream& operator<<(std::ostream& out, const PointClasses& classes);

/**
  Extract clusters of points from input PointView.

  For each point, find neighbors within a given tolerance (Euclidean distance).
  If a neighbor already belongs to another cluster, skip it. Otherwise, add it
  to the current cluster. Recursively visit newly added cluster points, looking
  for neighbors to add to the cluster.

  \param[in] view the input PointView.
  \param[in] min_points the minimum number of points in a cluster.
  \param[in] max_points the maximum number of points in a cluster.
  \param[in] tolerance the tolerance for adding points to a cluster.
  \returns a deque of clusters (themselves vectors of PointIds).
*/
template <class KDINDEX>
PDAL_EXPORT std::deque<PointIdList>
extractClusters(PointView& view, uint64_t min_points, uint64_t max_points,
                double tolerance)
{
    // The KD index type only selects a 2D or 3D distance metric; the region
    // growing itself is routed through the Rust C ABI.
    const bool is3d = std::is_same<KDINDEX, KD3Index>::value;

    const size_t count = view.size();
    std::vector<double> xyz(count * 3);
    for (PointId i = 0; i < count; ++i)
    {
        xyz[3 * i] = view.getFieldAs<double>(Dimension::Id::X, i);
        xyz[3 * i + 1] = view.getFieldAs<double>(Dimension::Id::Y, i);
        xyz[3 * i + 2] = view.getFieldAs<double>(Dimension::Id::Z, i);
    }

    uint64_t* clusterSizes = nullptr;
    uint64_t clusterCount = 0;
    uint64_t* pointIds = nullptr;
    uint64_t pointCount = 0;
    pdal_segmentation_extract_clusters(xyz.data(), count, min_points,
                                       max_points, tolerance, is3d,
                                       &clusterSizes, &clusterCount, &pointIds,
                                       &pointCount);

    std::deque<PointIdList> clusters;
    uint64_t offset = 0;
    for (uint64_t c = 0; c < clusterCount; ++c)
    {
        PointIdList cluster;
        for (uint64_t k = 0; k < clusterSizes[c]; ++k)
            cluster.push_back(static_cast<PointId>(pointIds[offset++]));
        clusters.push_back(std::move(cluster));
    }

    pdal_u64_array_free(clusterSizes, clusterCount);
    pdal_u64_array_free(pointIds, pointCount);
    return clusters;
}

PDAL_EXPORT void ignoreDimRange(DimRange dr, PointViewPtr input,
                                PointViewPtr keep, PointViewPtr ignore);
PDAL_EXPORT void ignoreDimRanges(std::vector<DimRange>& ranges,
                                 PointViewPtr input, PointViewPtr keep,
                                 PointViewPtr ignore);

PDAL_EXPORT void ignoreClassBits(PointViewPtr input, PointViewPtr keep,
                                 PointViewPtr ignore, PointClasses classbits);

PDAL_EXPORT void segmentLastReturns(PointViewPtr input, PointViewPtr last,
                                    PointViewPtr other);

PDAL_EXPORT void segmentReturns(PointViewPtr input, PointViewPtr first,
                                PointViewPtr second, StringList returns);

PDAL_EXPORT PointIdList farthestPointSampling(PointView& view,
                                              point_count_t count);

} // namespace Segmentation
} // namespace pdal
