/******************************************************************************
 * Copyright (c) 2026, Hobu Inc.
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

#include <pdal/pdal_test_main.hpp>

#include <istream>
#include <ostream>
#include <vector>

#include <pdal/util/Charbuf.hpp>

namespace pdal
{

TEST(CharbufTest, read_and_seek)
{
    std::vector<char> data{'a', 'b', 'c', 'd', 'e'};
    Charbuf buf(data);
    std::istream in(&buf);

    char c;
    in.get(c);
    EXPECT_EQ(c, 'a');

    in.seekg(3);
    in.get(c);
    EXPECT_EQ(c, 'd');

    in.seekg(-2, std::ios_base::end);
    in.get(c);
    EXPECT_EQ(c, 'd');

    in.clear();
    in.seekg(2, std::ios_base::beg);
    in.get(c);
    EXPECT_EQ(c, 'c');

    in.clear();
    in.seekg(1, std::ios_base::cur);
    in.get(c);
    EXPECT_EQ(c, 'e');

    in.clear();
    in.seekg(6);
    EXPECT_TRUE(in.fail());
}

TEST(CharbufTest, write_and_seek_with_offset)
{
    std::vector<char> data{'a', 'b', 'c', 'd', 'e'};
    Charbuf buf(data, 10);
    std::ostream out(&buf);

    out.seekp(12);
    out.put('X');
    EXPECT_TRUE(out.good());
    EXPECT_EQ(data[2], 'X');

    out.seekp(15);
    EXPECT_TRUE(out.good());
    out.put('Y');
    EXPECT_FALSE(out.good());
    EXPECT_EQ(data[4], 'e');
}

TEST(CharbufTest, seekoff_honors_buffer_offset)
{
    std::vector<char> data{'a', 'b', 'c', 'd', 'e'};
    Charbuf buf(data, 10);
    std::iostream stream(&buf);

    char c;
    stream.seekg(12, std::ios_base::beg);
    stream.get(c);
    EXPECT_EQ(c, 'c');

    stream.seekp(13, std::ios_base::beg);
    stream.put('X');
    EXPECT_TRUE(stream.good());
    EXPECT_EQ(data[3], 'X');
}

} // namespace pdal
