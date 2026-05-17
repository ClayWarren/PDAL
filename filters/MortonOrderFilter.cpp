/******************************************************************************
 * Copyright (c) 2014, Bradley J Chambers (brad.chambers@gmail.com)
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
 *       notice, this list of conditions and the following disclaimer in * the
 *documentation and/or other materials provided with the distribution.
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

#include "MortonOrderFilter.hpp"
#include "private/RustViewConverter.hpp"

#include <climits>
#include <iostream>
#include <limits>
#include <map>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.mortonorder",
    "Morton or z-order sorting of points. See "
    "http://en.wikipedia.org/wiki/Z-order_curve for more detail.",
    "https://pdal.org/stages/filters.mortonorder.html"};

CREATE_STATIC_STAGE(MortonOrderFilter, s_info)

std::string MortonOrderFilter::getName() const
{
    return s_info.name;
}

void MortonOrderFilter::addArgs(ProgramArgs& args)
{
    args.add("reverse", "Reverse Morton", m_reverse, false);
}

// This used to be a lambda, but the VS compiler exploded, I guess.
typedef std::pair<double, double> Coord;
namespace
{
bool less_msb(const int& x, const int& y)
{
    return x < y && x < (x ^ y);
}

class CmpZOrder
{
public:
    bool operator()(const Coord& c1, const Coord& c2) const
    {
        int a[2] = {(int)(c1.first * INT_MAX), (int)(c1.second * INT_MAX)};
        int b[2] = {(int)(c2.first * INT_MAX), (int)(c2.second * INT_MAX)};

        int j = 0;
        int x = 0;

        for (int k = 0; k < 2; k++)
        {
            int y = a[k] ^ b[k];
            if (less_msb(x, y))
            {
                j = k;
                x = y;
            }
        }
        return (a[j] - b[j]) < 0;
    };
};
} // namespace

class ReverseZOrder
{
public:
    static uint32_t encode_morton(uint32_t x, uint32_t y)
    {
        return (part1_by1(y) << 1) + part1_by1(x);
    }

    static uint32_t reverse_morton(uint32_t index)
    {
        index = ((index >> 1) & 0x55555555u) | ((index & 0x55555555u) << 1);
        index = ((index >> 2) & 0x33333333u) | ((index & 0x33333333u) << 2);
        index = ((index >> 4) & 0x0f0f0f0fu) | ((index & 0x0f0f0f0fu) << 4);
        index = ((index >> 8) & 0x00ff00ffu) | ((index & 0x00ff00ffu) << 8);
        index = ((index >> 16) & 0xffffu) | ((index & 0xffffu) << 16);
        return index;
    }

private:
    static uint32_t part1_by1(uint32_t x)
    {
        x &= 0x0000ffff;
        x = (x ^ (x << 8)) & 0x00ff00ff;
        x = (x ^ (x << 4)) & 0x0f0f0f0f;
        x = (x ^ (x << 2)) & 0x33333333;
        x = (x ^ (x << 1)) & 0x55555555;
        return x;
    }

    static uint32_t part1_by2(uint32_t x)
    {
        x &= 0x000003ff;
        x = (x ^ (x << 16)) & 0xff0000ff;
        x = (x ^ (x << 8)) & 0x0300f00f;
        x = (x ^ (x << 4)) & 0x030c30c3;
        x = (x ^ (x << 2)) & 0x09249249;
        return x;
    }
};


PointViewSet MortonOrderFilter::run(PointViewPtr inView)
{
    PointViewSet viewSet;

    pdal_stage_t* stage = pdal_stage_create_mortonorder(m_reverse);
    if (!stage)
        throwError("Failed to create Rust mortonorder stage.");

    pdal_point_view_t* rust_in = rust_view_converter::toRust(inView);
    pdal_point_view_t* rust_out = pdal_stage_run(stage, rust_in);

    PointViewPtr outView = rust_view_converter::fromRust(rust_out, inView);
    viewSet.insert(outView);

    if (rust_out)
        pdal_point_view_destroy(rust_out);
    pdal_point_view_destroy(rust_in);
    pdal_stage_destroy(stage);

    return viewSet;
}

} // namespace pdal
