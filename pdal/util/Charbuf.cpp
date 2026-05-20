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

#include <pdal/util/Charbuf.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

namespace pdal
{

namespace
{

uint8_t seekdirId(std::ios_base::seekdir dir)
{
    if (dir == std::ios::beg)
        return 0;
    if (dir == std::ios::cur)
        return 1;
    if (dir == std::ios::end)
        return 2;
    return 255;
}

} // unnamed namespace

void Charbuf::initialize(char* buf, size_t count, std::ios::pos_type bufOffset)
{
    m_bufOffset = bufOffset;
    m_buf = buf;
    setg(buf, buf, buf + count);
    setp(buf, buf + count);
}

std::ios::pos_type Charbuf::seekpos(std::ios::pos_type pos,
                                    std::ios_base::openmode which)
{
    int64_t adjusted = pdal_charbuf_seekpos(
        static_cast<std::streamoff>(pos),
        static_cast<std::streamoff>(m_bufOffset), egptr() - eback(), false);
    if (which & std::ios_base::in)
    {
        if (adjusted < 0)
            return -1;
        char* cpos = eback() + adjusted;
        setg(eback(), cpos, egptr());
    }
    if (which & std::ios_base::out)
    {
        adjusted = pdal_charbuf_seekpos(
            static_cast<std::streamoff>(pos),
            static_cast<std::streamoff>(m_bufOffset), epptr() - m_buf, true);
        if (adjusted < 0)
            return -1;
        char* cpos = m_buf + adjusted;
        setp(cpos, epptr());
    }
    return adjusted;
}

std::ios::pos_type Charbuf::seekoff(std::ios::off_type off,
                                    std::ios_base::seekdir dir,
                                    std::ios_base::openmode which)
{
    std::ios::pos_type pos;
    const uint8_t dirId = seekdirId(dir);
    if (which & std::ios_base::in)
    {
        int64_t adjusted = pdal_charbuf_seekoff(
            off, dirId, static_cast<std::streamoff>(m_bufOffset),
            egptr() - eback(), gptr() - eback());
        if (adjusted < 0)
            return -1;
        char* cpos = eback() + adjusted;
        setg(eback(), cpos, egptr());
        pos = cpos - eback();
    }
    if (which & std::ios_base::out)
    {
        int64_t adjusted = pdal_charbuf_seekoff(
            off, dirId, static_cast<std::streamoff>(m_bufOffset),
            epptr() - m_buf, pptr() - m_buf);
        if (adjusted < 0)
            return -1;
        char* cpos = m_buf + adjusted;
        setp(cpos, epptr());
        pos = cpos - m_buf;
    }
    return pos;
}

} // namespace pdal
