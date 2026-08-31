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

#include <limits>

namespace pdal
{

namespace
{

bool relativeSeekPosition(std::ios::off_type off, std::ios_base::seekdir dir,
                          std::ios::off_type current, std::ios::off_type end,
                          std::ios::off_type bufferOffset,
                          std::ios::off_type& position)
{
    if (bufferOffset < 0 || end < 0 ||
        bufferOffset > (std::numeric_limits<std::ios::off_type>::max)() - end)
        return false;

    switch (dir)
    {
    case std::ios::beg:
        if (off < bufferOffset || off > bufferOffset + end)
            return false;
        position = off - bufferOffset;
        return true;
    case std::ios::cur:
        if (off < -current || off > end - current)
            return false;
        position = current + off;
        return true;
    case std::ios::end:
        if (off < -end || off > 0)
            return false;
        position = end + off;
        return true;
    default:
        return false;
    }
}

} // unnamed namespace

void Charbuf::initialize(char *buf, size_t count, std::ios::pos_type bufOffset)
{
    m_bufOffset = bufOffset;
    m_buf = buf;
    setg(buf, buf, buf + count);
    setp(buf, buf + count);
}


std::ios::pos_type Charbuf::seekpos(std::ios::pos_type pos,
    std::ios_base::openmode which)
{
    const bool input = (which & std::ios_base::in) != 0;
    const bool output = (which & std::ios_base::out) != 0;
    if (!input && !output)
        return -1;

    const std::ios::off_type absolute = static_cast<std::ios::off_type>(pos);
    const std::ios::off_type bufferOffset =
        static_cast<std::ios::off_type>(m_bufOffset);
    std::ios::off_type inputPos = -1;
    std::ios::off_type outputPos = -1;
    if (input &&
        !relativeSeekPosition(absolute, std::ios::beg, gptr() - eback(),
                              egptr() - eback(), bufferOffset, inputPos))
        return -1;
    if (output &&
        !relativeSeekPosition(absolute, std::ios::beg, pptr() - m_buf,
                              epptr() - m_buf, bufferOffset, outputPos))
        return -1;

    if (input)
        setg(eback(), eback() + inputPos, egptr());
    if (output)
        setp(m_buf + outputPos, epptr());
    return pos;
}

std::ios::pos_type
Charbuf::seekoff(std::ios::off_type off, std::ios_base::seekdir dir,
    std::ios_base::openmode which)
{
    const bool input = (which & std::ios_base::in) != 0;
    const bool output = (which & std::ios_base::out) != 0;
    if (!input && !output)
        return -1;

    const std::ios::off_type inputCurrent = input ? gptr() - eback() : 0;
    const std::ios::off_type outputCurrent = output ? pptr() - m_buf : 0;
    if (input && output && dir == std::ios::cur &&
        inputCurrent != outputCurrent)
        return -1;

    const std::ios::off_type bufferOffset =
        static_cast<std::ios::off_type>(m_bufOffset);
    std::ios::off_type inputPos = -1;
    std::ios::off_type outputPos = -1;
    if (input &&
        !relativeSeekPosition(off, dir, inputCurrent, egptr() - eback(),
                              bufferOffset, inputPos))
        return -1;
    if (output &&
        !relativeSeekPosition(off, dir, outputCurrent, epptr() - m_buf,
                              bufferOffset, outputPos))
        return -1;
    if (input && output && inputPos != outputPos)
        return -1;

    if (input)
        setg(eback(), eback() + inputPos, egptr());
    if (output)
        setp(m_buf + outputPos, epptr());

    const std::ios::off_type relative = input ? inputPos : outputPos;
    return std::ios::pos_type(bufferOffset + relative);
}

} //namespace
