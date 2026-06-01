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

#include <pdal/PointTable.hpp>
#include <pdal_capi.h>

namespace pdal
{

ColumnPointTable::ColumnPointTable()
    : SimplePointTable(m_layout),
      m_storage(reinterpret_cast<pdal_column_storage*>(
          pdal_column_storage_create(m_blockPtCnt)))
{
}

ColumnPointTable::~ColumnPointTable()
{
    pdal_column_storage_destroy(
        reinterpret_cast<pdal_column_storage_t*>(m_storage));
}

void ColumnPointTable::finalize()
{
    // finalize() must be idempotent: Stage::execute() finalizes the table a
    // second time after points have been added, and pdal_column_storage_set_
    // dimensions() resets storage to zero points. Re-running it would discard
    // already-stored data (and leave subsequent reads dereferencing null
    // slots). Guard on the layout's finalized flag like BasePointTable does.
    if (m_layoutRef.finalized())
        return;

    m_layoutRef.orderDimensions();
    const auto& dims = m_layoutRef.dims();
    m_dimSizes.clear();
    m_dimSizes.resize(dims.size(), 0);
    for (Dimension::Id id : dims)
    {
        const Dimension::Detail* d = m_layoutRef.dimDetail(id);
        m_dimSizes[d->order()] =
            static_cast<uint64_t>(Dimension::size(d->type()));
    }
    pdal_column_storage_set_dimensions(
        reinterpret_cast<pdal_column_storage_t*>(m_storage),
        m_dimSizes.data(), static_cast<uint64_t>(m_dimSizes.size()));

    m_layoutRef.finalize();
}

PointId ColumnPointTable::addPoint()
{
    return static_cast<PointId>(pdal_column_storage_add_point(
        reinterpret_cast<pdal_column_storage_t*>(m_storage)));
}

namespace
{

void copy(const char* src, char* dst, Dimension::Type type)
{
    switch (type)
    {
    case Dimension::Type::Double:
        *reinterpret_cast<double*>(dst) = *reinterpret_cast<const double*>(src);
        break;
    case Dimension::Type::Float:
        *reinterpret_cast<float*>(dst) = *reinterpret_cast<const float*>(src);
        break;
    case Dimension::Type::Signed8:
        *reinterpret_cast<int8_t*>(dst) = *reinterpret_cast<const int8_t*>(src);
        break;
    case Dimension::Type::Signed16:
        *reinterpret_cast<int16_t*>(dst) =
            *reinterpret_cast<const int16_t*>(src);
        break;
    case Dimension::Type::Signed32:
        *reinterpret_cast<int32_t*>(dst) =
            *reinterpret_cast<const int32_t*>(src);
        break;
    case Dimension::Type::Signed64:
        *reinterpret_cast<int64_t*>(dst) =
            *reinterpret_cast<const int64_t*>(src);
        break;
    case Dimension::Type::Unsigned8:
        *reinterpret_cast<uint8_t*>(dst) =
            *reinterpret_cast<const uint8_t*>(src);
        break;
    case Dimension::Type::Unsigned16:
        *reinterpret_cast<uint16_t*>(dst) =
            *reinterpret_cast<const uint16_t*>(src);
        break;
    case Dimension::Type::Unsigned32:
        *reinterpret_cast<uint32_t*>(dst) =
            *reinterpret_cast<const uint32_t*>(src);
        break;
    case Dimension::Type::Unsigned64:
        *reinterpret_cast<uint64_t*>(dst) =
            *reinterpret_cast<const uint64_t*>(src);
        break;
    case Dimension::Type::None:
    default:
        break;
    }
}

} // unnamed namespace

void ColumnPointTable::setFieldInternal(Dimension::Id dim, PointId idx,
                                        const void* src)
{
    const Dimension::Detail* d = m_layoutRef.dimDetail(dim);
    const uint64_t size = static_cast<uint64_t>(Dimension::size(d->type()));
    char* dst = static_cast<char*>(pdal_column_storage_dim_slot(
        reinterpret_cast<pdal_column_storage_t*>(m_storage),
        static_cast<uint64_t>(d->order()), size, static_cast<uint64_t>(idx)));

    copy(reinterpret_cast<const char*>(src), dst, d->type());
}

void ColumnPointTable::getFieldInternal(Dimension::Id dim, PointId idx,
                                        void* dst) const
{
    const Dimension::Detail* d = m_layoutRef.dimDetail(dim);
    const uint64_t size = static_cast<uint64_t>(Dimension::size(d->type()));
    const char* src = static_cast<const char*>(pdal_column_storage_dim_slot(
        reinterpret_cast<pdal_column_storage_t*>(m_storage),
        static_cast<uint64_t>(d->order()), size, static_cast<uint64_t>(idx)));

    copy(src, reinterpret_cast<char*>(dst), d->type());
}

char* ColumnPointTable::getDimension(const Dimension::Detail* d, PointId idx)
{
    const uint64_t size = static_cast<uint64_t>(Dimension::size(d->type()));
    return static_cast<char*>(pdal_column_storage_dim_slot(
        reinterpret_cast<pdal_column_storage_t*>(m_storage),
        static_cast<uint64_t>(d->order()), size, static_cast<uint64_t>(idx)));
}

const char* ColumnPointTable::getDimension(const Dimension::Detail* d,
                                           PointId idx) const
{
    ColumnPointTable* ncThis = const_cast<ColumnPointTable*>(this);
    return ncThis->getDimension(d, idx);
}

} // namespace pdal
