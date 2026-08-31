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
 *     * Neither the name of the Martin Isenburg or Iowa Department
 *       of Natural Resources nor the names of its contributors may be
 *       used to endorse or promote products derived from this software
 *       without specific prior written permission.
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

#ifdef _MSC_VER
#pragma warning (push)
#pragma warning (disable: 4251)
#endif

#include <lazperf/lazperf.hpp>

#ifdef _MSC_VER
#pragma warning (pop)
#endif

#include <cstring>
#include <limits>

#include <io/LasReader.hpp>
#include <io/private/las/Header.hpp>

#include "../connector/Connector.hpp"
#include "Tile.hpp"

namespace pdal
{
namespace copc
{

void Tile::read()
{
    try
    {
        if (m_entry.m_byteSize <= 0)
            throw pdal_error("Invalid COPC tile byte size.");
        if (m_entry.m_pointCount <= 0 ||
            static_cast<uint64_t>(m_entry.m_pointCount) > m_header.pointCount())
            throw pdal_error("Invalid COPC tile point count.");

        const int basePointSize = m_header.baseCount();
        if (basePointSize <= 0 || m_header.pointSize < basePointSize)
            throw pdal_error("Invalid COPC point size.");

        const size_t pointSize = m_header.pointSize;
        const size_t pointCount = static_cast<size_t>(m_entry.m_pointCount);
        if (pointCount > (std::numeric_limits<size_t>::max)() / pointSize)
            throw pdal_error("Invalid COPC tile size.");

        std::vector<char> buf =
            m_connector.getBinary(m_entry.m_offset, m_entry.m_byteSize);
        size_t pos = 0;
        lazperf::InputCb cb = [&buf, &pos](unsigned char* dest, size_t size)
        {
            if (size > buf.size() - pos)
                throw pdal_error("Invalid or truncated COPC tile data.");
            std::memcpy(dest, buf.data() + pos, size);
            pos += size;
        };
        lazperf::las_decompressor::ptr d = lazperf::build_las_decompressor(
            cb, m_header.pointFormat(), m_header.ebCount());
        if (!d)
            throw pdal_error("Invalid COPC point format.");

        // Resize our vector to accommodate the decompressed data.
        m_data.resize(pointCount * pointSize);

        int32_t cnt = m_entry.m_pointCount;
        char *p = m_data.data();
        while (cnt--)
        {
            d->decompress(p);
            p += pointSize;
        }
    }
    catch (const std::exception& ex)
    {
        m_error = ex.what();
    }
    catch (...)
    {
        m_error = "Unknown exception when reading tile contents";
    }
}

} // namespace copc
} // namespace pdal
