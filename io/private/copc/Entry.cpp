/******************************************************************************
 * Copyright (c) 2021, Hobu Inc.
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

#include "Entry.hpp"

#include <pdal_capi.h>

namespace pdal
{
namespace copc
{

Hierarchy::Hierarchy(const std::vector<char>& data)
{
    pdal_copc_entry_t* entries = nullptr;
    uint64_t count = 0;
    if (!pdal_copc_hierarchy_parse(
            reinterpret_cast<const uint8_t*>(data.data()), data.size(),
            &entries, &count))
        return;

    for (uint64_t i = 0; i < count; ++i)
    {
        const pdal_copc_entry_t& in = entries[i];
        Entry e(Key(in.d, in.x, in.y, in.z), in.offset, in.byte_size,
                in.point_count);
        m_entries.insert(e);
    }
    pdal_copc_entries_free(entries, count);
}

point_count_t Hierarchy::pointCount() const
{
    point_count_t pointCount = 0;

    for (const Entry& e : m_entries)
        pointCount += e.m_pointCount;
    return pointCount;
}

} // namespace copc
} // namespace pdal
