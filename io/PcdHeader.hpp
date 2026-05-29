/******************************************************************************
 * Copyright (c) 2019, Kirk McKelvey (kirkoman@gmail.com)
 * Copyright (c) 2019, Bradley J Chambers (brad.chambers@gmail.com)
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

namespace pdal
{

// PCD field/dimension typing shared by the (Rust-delegating) PCD writer. The
// header parsing and serialization that once lived alongside these types moved
// into the Rust PCD reader/writer behind the C ABI, so only the dimension
// descriptors remain on the C++ side.
enum class PcdFieldType
{
    unknown,
    I,
    U,
    F
};

struct PcdField
{
    PcdField()
        : m_id(Dimension::Id::Unknown), m_size(4),
          m_type(PcdFieldType::unknown), m_count(1)
    {
    }

    PcdField(std::string& label) : PcdField()
    {
        m_id = Dimension::id(label);
        m_label = label;
    }

    std::string m_label;
    Dimension::Id m_id;
    uint32_t m_size;
    PcdFieldType m_type;
    uint32_t m_count;
};

} // namespace pdal
