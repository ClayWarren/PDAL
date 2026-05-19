/******************************************************************************
 * Copyright (c) 2014, Connor Manning (connor@hobu.co)
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

#include <cmath>
#include <limits>
#include <memory>

#include <pdal/PointView.hpp>
#include <pdal/QuadIndex.hpp>
#include <pdal/util/Utils.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

namespace
{
using namespace pdal;

struct BBox
{
    BBox(Point minimum, Point maximum)
        : minimum(minimum), maximum(maximum),
          center(minimum.x + (maximum.x - minimum.x) / 2,
                 minimum.y + (maximum.y - minimum.y) / 2),
          halfWidth(center.x - minimum.x), halfHeight(center.y - minimum.y)
    {
    }

    BBox(const BBox& other)
        : minimum(other.minimum), maximum(other.maximum), center(other.center),
          halfWidth(other.halfWidth), halfHeight(other.halfHeight)
    {
    }

    // Returns true if this BBox shares any area in common with another.
    bool overlaps(const BBox& other) const
    {
        return std::abs(center.x - other.center.x) <
                   halfWidth + other.halfWidth &&
               std::abs(center.y - other.center.y) <
                   halfHeight + other.halfHeight;
    }

    bool overlaps(const double xBegin, const double xEnd, const double yBegin,
                  const double yEnd) const
    {
        const BBox other(Point(xBegin, yBegin), Point(xEnd, yEnd));

        return overlaps(other);
    }

    // Returns true if the requested point is contained within this BBox.
    bool contains(const Point& p) const
    {
        return p.x >= minimum.x && p.y >= minimum.y && p.x < maximum.x &&
               p.y < maximum.y;
    }

    const Point minimum;
    const Point maximum;

    // Pre-calculate these properties, rather than exposing functions to
    // calculate them on-demand, due to the large number of times that
    // these will be needed when querying the quad tree.
    const Point center;
    const double halfWidth;
    const double halfHeight;

    BBox& operator=(const BBox&); // not implemented
};

} // anonymous namespace

namespace pdal
{

// Recursive quadtree implementation.
struct Tree
{
    Tree(BBox bbox, const QuadPointRef* data = nullptr)
        : bbox(bbox), data(data), nw(), ne(), se(), sw()
    {
    }

    void getFills(std::vector<std::size_t>& fills, std::size_t level = 0) const;

    // Returns depth resulting from the insertion of this point.
    std::size_t addPoint(const QuadPointRef* toAdd, std::size_t curDepth = 0);

    void getPoints(PointIdList& results, std::size_t depthBegin,
                   std::size_t depthEnd, std::size_t curDepth) const;

    void getPoints(PointIdList& results, std::size_t rasterize, double xBegin,
                   double xEnd, double xStep, double yBegin, double yEnd,
                   double yStep, std::size_t curDepth) const;

    void getPoints(PointIdList& results, double xBegin, double xEnd,
                   double xStep, double yBegin, double yEnd,
                   double yStep) const;

    void getPoints(PointIdList& results, const BBox& query,
                   std::size_t depthBegin, std::size_t depthEnd,
                   std::size_t curDepth) const;

    const BBox bbox;
    const QuadPointRef* data;

    std::unique_ptr<Tree> nw;
    std::unique_ptr<Tree> ne;
    std::unique_ptr<Tree> se;
    std::unique_ptr<Tree> sw;
};

std::size_t Tree::addPoint(const QuadPointRef* toAdd,
                           const std::size_t curDepth)
{
    if (data)
    {
        const Point& center(bbox.center);

        if (toAdd->point.sqDist(center) < data->point.sqDist(center))
        {
            std::swap(data, toAdd);
        }

        const std::size_t nextDepth(curDepth + 1);

        if (toAdd->point.x < center.x)
        {
            if (toAdd->point.y < center.y)
            {
                if (sw)
                {
                    return sw->addPoint(toAdd, nextDepth);
                }
                else
                {
                    sw.reset(
                        new Tree(BBox(Point(bbox.minimum.x, bbox.minimum.y),
                                      Point(center.x, center.y)),
                                 toAdd));

                    return nextDepth;
                }
            }
            else
            {
                if (nw)
                {
                    return nw->addPoint(toAdd, nextDepth);
                }
                else
                {
                    nw.reset(new Tree(BBox(Point(bbox.minimum.x, center.y),
                                           Point(center.x, bbox.maximum.y)),
                                      toAdd));

                    return nextDepth;
                }
            }
        }
        else
        {
            if (toAdd->point.y < center.y)
            {
                if (se)
                {
                    return se->addPoint(toAdd, nextDepth);
                }
                else
                {
                    se.reset(new Tree(BBox(Point(center.x, bbox.minimum.y),
                                           Point(bbox.maximum.x, center.y)),
                                      toAdd));

                    return nextDepth;
                }
            }
            else
            {
                if (ne)
                {
                    return ne->addPoint(toAdd, nextDepth);
                }
                else
                {
                    ne.reset(
                        new Tree(BBox(Point(center.x, center.y),
                                      Point(bbox.maximum.x, bbox.maximum.y)),
                                 toAdd));

                    return nextDepth;
                }
            }
        }
    }
    else
    {
        data = toAdd;
        return curDepth;
    }
}

// Fills are a count of the number of points at each level of the quad tree.
void Tree::getFills(std::vector<std::size_t>& fills, std::size_t level) const
{
    if (data)
    {
        if (level >= fills.size())
            fills.resize(level + 1);
        (fills[level])++;
    }

    ++level;
    if (nw)
        nw->getFills(fills, level);
    if (ne)
        ne->getFills(fills, level);
    if (sw)
        sw->getFills(fills, level);
    if (se)
        se->getFills(fills, level);
}

void Tree::getPoints(PointIdList& results, const std::size_t depthBegin,
                     const std::size_t depthEnd, std::size_t curDepth) const
{
    if (data && curDepth >= depthBegin)
    {
        results.push_back(data->pbIndex);
    }

    if (++curDepth < depthEnd || depthEnd == 0)
    {
        if (nw)
            nw->getPoints(results, depthBegin, depthEnd, curDepth);
        if (ne)
            ne->getPoints(results, depthBegin, depthEnd, curDepth);
        if (se)
            se->getPoints(results, depthBegin, depthEnd, curDepth);
        if (sw)
            sw->getPoints(results, depthBegin, depthEnd, curDepth);
    }
}

void Tree::getPoints(PointIdList& results, const std::size_t rasterize,
                     const double xBegin, const double xEnd, const double xStep,
                     const double yBegin, const double yEnd, const double yStep,
                     std::size_t curDepth) const
{
    if (curDepth == rasterize)
    {
        if (data)
        {
            double xOffset(Utils::sround((bbox.center.x - xBegin) / xStep));
            double yOffset(Utils::sround((bbox.center.y - yBegin) / yStep));

            const std::size_t index(static_cast<size_t>(
                Utils::sround(yOffset * (xEnd - xBegin) / xStep + xOffset)));

            results.at(index) = data->pbIndex;
        }
    }
    else if (++curDepth <= rasterize)
    {
        if (nw)
            nw->getPoints(results, rasterize, xBegin, xEnd, xStep, yBegin, yEnd,
                          yStep, curDepth);

        if (ne)
            ne->getPoints(results, rasterize, xBegin, xEnd, xStep, yBegin, yEnd,
                          yStep, curDepth);

        if (se)
            se->getPoints(results, rasterize, xBegin, xEnd, xStep, yBegin, yEnd,
                          yStep, curDepth);

        if (sw)
            sw->getPoints(results, rasterize, xBegin, xEnd, xStep, yBegin, yEnd,
                          yStep, curDepth);
    }
}

void Tree::getPoints(PointIdList& results, const double xBegin,
                     const double xEnd, const double xStep, const double yBegin,
                     const double yEnd, const double yStep) const
{
    if (!bbox.overlaps(xBegin, xEnd, yBegin, yEnd))
    {
        return;
    }

    if (nw)
        nw->getPoints(results, xBegin, xEnd, xStep, yBegin, yEnd, yStep);

    if (ne)
        ne->getPoints(results, xBegin, xEnd, xStep, yBegin, yEnd, yStep);

    if (se)
        se->getPoints(results, xBegin, xEnd, xStep, yBegin, yEnd, yStep);

    if (sw)
        sw->getPoints(results, xBegin, xEnd, xStep, yBegin, yEnd, yStep);

    // Add data after calling child nodes so we prefer upper levels of the tree.
    if (data && data->point.x >= xBegin && data->point.y >= yBegin &&
        data->point.x < xEnd - xStep && data->point.y < yEnd - yStep)
    {
        double xOffset(Utils::sround((data->point.x - xBegin) / xStep));
        double yOffset(Utils::sround((data->point.y - yBegin) / yStep));

        std::size_t index(static_cast<size_t>(
            Utils::sround(yOffset * (xEnd - xBegin) / xStep + xOffset)));

        if (index < results.size())
        {
            results.at(index) = data->pbIndex;
        }
    }
}

void Tree::getPoints(PointIdList& results, const BBox& query,
                     const std::size_t depthBegin, const std::size_t depthEnd,
                     std::size_t curDepth) const
{
    if (!query.overlaps(bbox))
    {
        return;
    }

    if (data && query.contains(data->point) && curDepth >= depthBegin &&
        (curDepth < depthEnd || depthEnd == 0))
    {
        results.push_back(data->pbIndex);
    }

    if (++curDepth < depthEnd || depthEnd == 0)
    {
        if (nw)
            nw->getPoints(results, query, depthBegin, depthEnd, curDepth);
        if (ne)
            ne->getPoints(results, query, depthBegin, depthEnd, curDepth);
        if (se)
            se->getPoints(results, query, depthBegin, depthEnd, curDepth);
        if (sw)
            sw->getPoints(results, query, depthBegin, depthEnd, curDepth);
    }
}

struct QuadIndex::QImpl
{
    QImpl(const PointView& view, std::size_t topLevel);
    QImpl(const PointView& view, double xMin, double yMin, double xMax,
          double yMax, std::size_t topLevel);
    QImpl(const std::vector<std::shared_ptr<QuadPointRef>>& points, double xMin,
          double yMin, double xMax, double yMax, std::size_t topLevel);
    ~QImpl();

    void getBounds(double& xMin, double& yMin, double& xMax,
                   double& yMax) const;

    std::size_t getDepth() const;

    std::vector<std::size_t> getFills();

    PointIdList getPoints(std::size_t depthBegin, std::size_t depthEnd) const;

    PointIdList getPoints(std::size_t rasterize, double& xBegin, double& xEnd,
                          double& xStep, double& yBegin, double& yEnd,
                          double& yStep) const;

    PointIdList getPoints(double xBegin, double xEnd, double xStep,
                          double yBegin, double yEnd, double yStep) const;

    PointIdList getPoints(double xMin, double yMin, double xMax, double yMax,
                          std::size_t depthBegin, std::size_t depthEnd) const;

    pdal_quad_index_t* m_index;
};

QuadIndex::QImpl::QImpl(const PointView& view, std::size_t topLevel)
    : m_index(nullptr)
{
    double xMin((std::numeric_limits<double>::max)());
    double yMin((std::numeric_limits<double>::max)());
    double xMax((std::numeric_limits<double>::lowest)());
    double yMax((std::numeric_limits<double>::lowest)());
    std::vector<double> xs(view.size());
    std::vector<double> ys(view.size());
    std::vector<uint64_t> ids(view.size());

    for (PointId i(0); i < view.size(); ++i)
    {
        xs[i] = view.getFieldAs<double>(Dimension::Id::X, i);
        ys[i] = view.getFieldAs<double>(Dimension::Id::Y, i);
        ids[i] = i;
        xMin = (std::min)(xMin, xs[i]);
        yMin = (std::min)(yMin, ys[i]);
        xMax = (std::max)(xMax, xs[i]);
        yMax = (std::max)(yMax, ys[i]);
    }

    m_index =
        pdal_quad_index_create(xs.data(), ys.data(), ids.data(), xs.size(),
                               xMin, yMin, xMax, yMax, topLevel);
}

QuadIndex::QImpl::QImpl(const PointView& view, double xMin, double yMin,
                        double xMax, double yMax, std::size_t topLevel)
    : m_index(nullptr)
{
    std::vector<double> xs(view.size());
    std::vector<double> ys(view.size());
    std::vector<uint64_t> ids(view.size());

    for (PointId i(0); i < view.size(); ++i)
    {
        xs[i] = view.getFieldAs<double>(Dimension::Id::X, i);
        ys[i] = view.getFieldAs<double>(Dimension::Id::Y, i);
        ids[i] = i;
    }

    m_index =
        pdal_quad_index_create(xs.data(), ys.data(), ids.data(), xs.size(),
                               xMin, yMin, xMax, yMax, topLevel);
}

QuadIndex::QImpl::QImpl(
    const std::vector<std::shared_ptr<QuadPointRef>>& points, double xMin,
    double yMin, double xMax, double yMax, std::size_t topLevel)
    : m_index(nullptr)
{
    std::vector<double> xs(points.size());
    std::vector<double> ys(points.size());
    std::vector<uint64_t> ids(points.size());

    for (std::size_t i = 0; i < points.size(); ++i)
    {
        xs[i] = points[i]->point.x;
        ys[i] = points[i]->point.y;
        ids[i] = points[i]->pbIndex;
    }

    m_index =
        pdal_quad_index_create(xs.data(), ys.data(), ids.data(), xs.size(),
                               xMin, yMin, xMax, yMax, topLevel);
}

QuadIndex::QImpl::~QImpl()
{
    pdal_quad_index_destroy(m_index);
}

void QuadIndex::QImpl::getBounds(double& xMin, double& yMin, double& xMax,
                                 double& yMax) const
{
    pdal_bounds2d_t bounds;
    pdal_quad_index_bounds(m_index, &bounds);
    xMin = bounds.minx;
    yMin = bounds.miny;
    xMax = bounds.maxx;
    yMax = bounds.maxy;
}

std::size_t QuadIndex::QImpl::getDepth() const
{
    return pdal_quad_index_depth(m_index);
}

std::vector<std::size_t> QuadIndex::QImpl::getFills()
{
    uint64_t len = 0;
    uint64_t* fills = pdal_quad_index_fills(m_index, &len);
    std::vector<std::size_t> output(len);
    for (uint64_t i = 0; i < len; ++i)
        output[i] = fills[i];
    pdal_u64_array_free(fills, len);
    return output;
}

PointIdList QuadIndex::QImpl::getPoints(const std::size_t minDepth,
                                        const std::size_t maxDepth) const
{
    uint64_t len = 0;
    uint64_t* ids =
        pdal_quad_index_points_by_depth(m_index, minDepth, maxDepth, &len);
    PointIdList results(ids, ids + len);
    pdal_u64_array_free(ids, len);
    return results;
}

PointIdList QuadIndex::QImpl::getPoints(const std::size_t rasterize,
                                        double& xBegin, double& xEnd,
                                        double& xStep, double& yBegin,
                                        double& yEnd, double& yStep) const
{
    uint64_t len = 0;
    uint64_t* ids = pdal_quad_index_points_raster_level(
        m_index, rasterize, &xBegin, &xEnd, &xStep, &yBegin, &yEnd, &yStep,
        &len);
    PointIdList results(ids, ids + len);
    pdal_u64_array_free(ids, len);
    return results;
}

PointIdList QuadIndex::QImpl::getPoints(const double xBegin, const double xEnd,
                                        const double xStep, const double yBegin,
                                        const double yEnd,
                                        const double yStep) const
{
    uint64_t len = 0;
    uint64_t* ids = pdal_quad_index_points_raster_bounds(
        m_index, xBegin, xEnd, xStep, yBegin, yEnd, yStep, &len);
    PointIdList results(ids, ids + len);
    pdal_u64_array_free(ids, len);
    return results;
}

PointIdList QuadIndex::QImpl::getPoints(double xMin, double yMin, double xMax,
                                        double yMax, std::size_t minDepth,
                                        std::size_t maxDepth) const
{
    uint64_t len = 0;
    uint64_t* ids = pdal_quad_index_points_in_bounds(
        m_index, xMin, yMin, xMax, yMax, minDepth, maxDepth, &len);
    PointIdList results(ids, ids + len);
    pdal_u64_array_free(ids, len);
    return results;
}

QuadIndex::QuadIndex(const PointView& view, std::size_t topLevel)
    : m_qImpl(new QImpl(view, topLevel))
{
}

QuadIndex::QuadIndex(const PointView& view, double xMin, double yMin,
                     double xMax, double yMax, std::size_t topLevel)
    : m_qImpl(new QImpl(view, xMin, yMin, xMax, yMax, topLevel))
{
}

QuadIndex::QuadIndex(const std::vector<std::shared_ptr<QuadPointRef>>& points,
                     double xMin, double yMin, double xMax, double yMax,
                     std::size_t topLevel)
    : m_qImpl(new QImpl(points, xMin, yMin, xMax, yMax, topLevel))
{
}

QuadIndex::~QuadIndex() {}

void QuadIndex::getBounds(double& xMin, double& yMin, double& xMax,
                          double& yMax) const
{
    m_qImpl->getBounds(xMin, yMin, xMax, yMax);
}

std::size_t QuadIndex::getDepth() const
{
    return m_qImpl->getDepth();
}

std::vector<std::size_t> QuadIndex::getFills() const
{
    return m_qImpl->getFills();
}

PointIdList QuadIndex::getPoints(std::size_t depthEnd) const
{
    return m_qImpl->getPoints(0, depthEnd);
}

PointIdList QuadIndex::getPoints(std::size_t depthBegin,
                                 std::size_t depthEnd) const
{
    return m_qImpl->getPoints(depthBegin, depthEnd);
}

PointIdList QuadIndex::getPoints(const std::size_t rasterize, double& xBegin,
                                 double& xEnd, double& xStep, double& yBegin,
                                 double& yEnd, double& yStep) const
{
    return m_qImpl->getPoints(rasterize, xBegin, xEnd, xStep, yBegin, yEnd,
                              yStep);
}

PointIdList QuadIndex::getPoints(const double xBegin, const double xEnd,
                                 const double xStep, const double yBegin,
                                 const double yEnd, const double yStep) const
{
    return m_qImpl->getPoints(xBegin, xEnd, xStep, yBegin, yEnd, yStep);
}

PointIdList QuadIndex::getPoints(double xMin, double yMin, double xMax,
                                 double yMax, std::size_t depthEnd) const
{
    return m_qImpl->getPoints(xMin, yMin, xMax, yMax, 0, depthEnd);
}

PointIdList QuadIndex::getPoints(double xMin, double yMin, double xMax,
                                 double yMax, std::size_t depthBegin,
                                 std::size_t depthEnd) const
{
    return m_qImpl->getPoints(xMin, yMin, xMax, yMax, depthBegin, depthEnd);
}

} // namespace pdal
