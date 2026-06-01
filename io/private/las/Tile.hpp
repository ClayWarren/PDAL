/******************************************************************************
 * Copyright (c) 2022, Hobu Inc.
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
 *     * Neither the name of Hobu, Inc. nor the
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

#include <pdal_capi.h>

namespace pdal
{
namespace las
{

class Tile
{
public:
    Tile(uint32_t chunk, uint32_t size)
        : m_tile(pdal_las_tile_create(chunk, size))
    {
    }
    ~Tile()
    {
        pdal_las_tile_destroy(m_tile);
    }

    const char* data() const
    {
        return pdal_las_tile_data_const(m_tile);
    }
    char* data()
    {
        return pdal_las_tile_data(m_tile);
    }
    size_t size() const
    {
        return pdal_las_tile_size(m_tile);
    }
    const char* pos() const
    {
        return pdal_las_tile_pos(m_tile);
    }
    uint32_t chunk() const
    {
        return pdal_las_tile_chunk(m_tile);
    }
    bool advance(int pointSize)
    {
        return pdal_las_tile_advance(m_tile, pointSize);
    }

private:
    pdal_las_tile_t* m_tile;
};
using TilePtr = std::unique_ptr<Tile>;

} // namespace las
} // namespace pdal
