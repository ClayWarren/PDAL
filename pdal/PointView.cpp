/******************************************************************************
 * Copyright (c) 2014, Hobu Inc.
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

#include <iomanip>
#include <numeric>

#include <pdal/KDIndex.hpp>
#include <pdal/PointView.hpp>
#include <pdal/util/Algorithm.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

#include "private/Raster.hpp"

namespace pdal
{

int PointView::m_lastId = 0;

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

} // unnamed namespace

PointView::PointView(PointTableRef pointTable)
    : m_pointTable(pointTable), m_layout(pointTable.layout()), m_size(0),
      m_id(0)
{
    m_id = ++m_lastId;
}

PointView::PointView(PointTableRef pointTable, const SpatialReference& srs)
    : m_pointTable(pointTable), m_layout(pointTable.layout()), m_size(0),
      m_id(0), m_spatialReference(srs)
{
    m_id = ++m_lastId;
}

PointView::~PointView() {}

PointViewIter PointView::begin()
{
    return PointViewIter(this, 0);
}

PointViewIter PointView::end()
{
    return PointViewIter(this, size());
}

PointId PointView::addPoint()
{
    PointId tableId = m_pointTable.addPoint();
    m_index.push_back(tableId);
    m_size++;
    return tableId;
}

template <typename Sorter> void PointView::basic_sort(Sorter sort, Compare comp)
{
    std::vector<PointId> order(size());

    std::iota(order.begin(), order.end(), 0);

    sort(order.begin(), order.end(), comp);

    for (PointId& o : order)
        o = m_index[o];
    for (size_t i = 0; i < m_index.size(); ++i)
        m_index[i] = order[i];
}

void PointView::sort(Dimension::Id dim)
{
    auto comp = [this, dim](PointId id1, PointId id2)
    { return compare(dim, id1, id2); };
    basic_sort(std::sort<std::vector<PointId>::iterator, PointView::Compare>,
               comp);
}

void PointView::sort(Compare comp)
{
    basic_sort(std::sort<std::vector<PointId>::iterator, PointView::Compare>,
               comp);
}

void PointView::stableSort(Dimension::Id dim)
{
    auto comp = [this, dim](PointId id1, PointId id2)
    { return compare(dim, id1, id2); };
    basic_sort(
        std::stable_sort<std::vector<PointId>::iterator, PointView::Compare>,
        comp);
}

void PointView::stableSort(Compare comp)
{
    basic_sort(
        std::stable_sort<std::vector<PointId>::iterator, PointView::Compare>,
        comp);
}

void PointView::calculateBounds(BOX2D& output) const
{
    pdal_point_view_t* rustView = toRustPointView(*this);
    pdal_bounds2d_t bounds;
    if (pdal_point_view_calculate_bounds_2d(rustView, &bounds))
    {
        output.minx = bounds.minx;
        output.maxx = bounds.maxx;
        output.miny = bounds.miny;
        output.maxy = bounds.maxy;
    }
    pdal_point_view_destroy(rustView);
}

void PointView::calculateBounds(BOX3D& output) const
{
    pdal_point_view_t* rustView = toRustPointView(*this);
    pdal_bounds3d_t bounds;
    if (pdal_point_view_calculate_bounds_3d(rustView, &bounds))
    {
        output.minx = bounds.minx;
        output.maxx = bounds.maxx;
        output.miny = bounds.miny;
        output.maxy = bounds.maxy;
        output.minz = bounds.minz;
        output.maxz = bounds.maxz;
    }
    pdal_point_view_destroy(rustView);
}

MetadataNode PointView::toMetadata() const
{
    MetadataNode node;

    const Dimension::IdList& dims = layout()->dims();

    for (PointId idx = 0; idx < size(); idx++)
    {
        MetadataNode pointnode = node.add(std::to_string(idx));
        for (auto di = dims.begin(); di != dims.end(); ++di)
        {
            double v = getFieldAs<double>(*di, idx);
            pointnode.add(layout()->dimName(*di), v);
        }
    }
    return node;
}

TriangularMesh* PointView::createMesh(const std::string& name)
{
    if (Utils::contains(m_meshes, name))
        return nullptr;
    auto res = m_meshes.insert(std::make_pair(
        name, std::unique_ptr<TriangularMesh>(new TriangularMesh)));
    if (res.second)
        return res.first->second.get();
    return nullptr;
}

TriangularMesh* PointView::mesh(const std::string& name)
{
    auto it = m_meshes.find(name);
    if (it != m_meshes.end())
        return it->second.get();
    if (name.empty() && m_meshes.size())
        return m_meshes.begin()->second.get();
    return nullptr;
}

Rasterd* PointView::createRaster(const std::string& name,
                                 const RasterLimits& limits, double nodata)
{
    if (Utils::contains(m_rasters, name))
        return nullptr;
    Rasterd* r = new Rasterd(limits, name, nodata);
    auto res =
        m_rasters.insert(std::make_pair(name, std::unique_ptr<Rasterd>(r)));
    if (res.second)
        return res.first->second.get();
    return nullptr;
}

Rasterd* PointView::raster(const std::string& name)
{
    auto it = m_rasters.find(name);
    if (it != m_rasters.end())
        return it->second.get();
    if (name.empty() && m_rasters.size())
        return m_rasters.begin()->second.get();
    return nullptr;
}

void PointView::invalidateProducts()
{
    m_index2.reset();
    m_index3.reset();
    // Should all meshes also be invalidated?
}

KD3Index& PointView::build3dIndex()
{
    // ABELL
    //  Should we allow a force of point view build - perhaps the index has
    //  changed or the point values have changed.
    if (!m_index3)
    {
        m_index3.reset(new KD3Index(*this));
        m_index3->build();
    }
    return *m_index3.get();
}

KD2Index& PointView::build2dIndex()
{
    // ABELL
    //  Should we allow a force of point view build - perhaps the index has
    //  changed or the point values have changed.
    if (!m_index2)
    {
        m_index2.reset(new KD2Index(*this));
        m_index2->build();
    }
    return *m_index2.get();
}

void PointView::dump(std::ostream& ostr) const
{
    using std::endl;
    PointLayoutPtr layout = m_pointTable.layout();
    const Dimension::IdList& dims = layout->dims();

    point_count_t numPoints = size();
    ostr << "Contains " << numPoints << "  points" << '\n';
    for (PointId idx = 0; idx < numPoints; idx++)
    {
        ostr << "Point: " << idx << '\n';

        for (auto di = dims.begin(); di != dims.end(); ++di)
        {
            Dimension::Id d = *di;
            const Dimension::Detail* dd = layout->dimDetail(d);
            ostr << layout->dimName(d) << " ("
                 << Dimension::interpretationName(dd->type()) << ") : ";

            switch (dd->type())
            {
            case Dimension::Type::Signed8:
            {
                ostr << (int)(getFieldAs<int8_t>(d, idx));
                break;
            }
            case Dimension::Type::Signed16:
            {
                ostr << getFieldAs<int16_t>(d, idx);
                break;
            }
            case Dimension::Type::Signed32:
            {
                ostr << getFieldAs<int32_t>(d, idx);
                break;
            }
            case Dimension::Type::Signed64:
            {
                ostr << getFieldAs<int64_t>(d, idx);
                break;
            }
            case Dimension::Type::Unsigned8:
            {
                ostr << (unsigned)(getFieldAs<uint8_t>(d, idx));
                break;
            }
            case Dimension::Type::Unsigned16:
            {
                ostr << getFieldAs<uint16_t>(d, idx);
                break;
            }
            case Dimension::Type::Unsigned32:
            {
                ostr << getFieldAs<uint32_t>(d, idx);
                break;
            }
            case Dimension::Type::Unsigned64:
            {
                ostr << getFieldAs<uint64_t>(d, idx);
                break;
            }
            case Dimension::Type::Float:
            {
                ostr << getFieldAs<float>(d, idx);
                break;
            }
            case Dimension::Type::Double:
            {
                ostr << getFieldAs<double>(d, idx);
                break;
            }
            case Dimension::Type::None:
                ostr << "NONE";
                break;
            }
            ostr << '\n';
        }
    }
}

std::ostream& operator<<(std::ostream& ostr, const PointView& buf)
{
    buf.dump(ostr);
    return ostr;
}

} // namespace pdal
