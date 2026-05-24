/******************************************************************************
 * Copyright (c) 2012, Michael P. Gerlek (mpg@flaxen.com)
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
 *     * Neither the name of Hobu, Inc. or Flaxen Consulting LLC nor the
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

#include "NitfReader.hpp"

#include <pdal_capi.h>

namespace pdal
{

static PluginInfo const s_info{"readers.nitf", "NITF Reader",
                               "https://pdal.org/stages/readers.nitf.html"};

CREATE_SHARED_STAGE(NitfReader, s_info)

std::string NitfReader::getName() const
{
    return s_info.name;
}

//
// References:
//   - NITF 2.1 standard: MIL-STD-2500C (01 May 2006)
//   - Lidar implementation profile v1.0 (2010-09-07)
//
// To be a proper lidar NITF file, the file must:
//   - have at least one Image segment ("IM")
//   - have at least one DES segment ("DE") named LIDARA
//

namespace
{

// Each call carries a `MetadataNode*` so we can route the Rust callback
// into PDAL's metadata tree.
extern "C" int append_nitf_metadata(const char* key, const char* value,
                                    void* userdata)
{
    auto* node = static_cast<MetadataNode*>(userdata);
    if (!node || !key || !value)
        return 0;
    node->add<std::string>(key, value);
    return 0;
}

} // namespace

void NitfReader::initialize(PointTableRef table)
{
    tryLoadRemote();

    uint64_t offset = 0;
    uint64_t length = 0;
    if (!pdal_nitf_lidar_segment(m_filename.c_str(), &offset, &length))
    {
        throwError(pdal_last_error());
    }
    m_offset = offset;
    m_length = length;
    setStartOffset(m_offset);

    if (!pdal_nitf_read_metadata(m_filename.c_str(), &append_nitf_metadata,
                                 &m_metadata))
    {
        throwError(pdal_last_error());
    }
    m_metadata.add("DESDATA_OFFSET", m_offset);
    m_metadata.add("DESDATA_LENGTH", m_length);

    // Initialize the LAS stuff with its own metadata node.
    MetadataNode lasNode = m_metadata.add(LasReader::getName());
    initializeLocal(table, lasNode);
}

} // namespace pdal
