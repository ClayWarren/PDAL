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

#include "GridPnp.hpp"

#include <pdal_capi.h>

namespace pdal
{

namespace
{

// Flatten the exterior ring (first) followed by interior rings into a single
// x,y coordinate buffer plus a per-ring vertex count, then build the Rust
// engine. Throws grid_error on any validation failure reported by Rust.
pdal_gridpnp_t* buildHandle(const GridPnp::Ring& outer,
                      const std::vector<GridPnp::Ring>& inners)
{
    std::vector<double> coords;
    std::vector<size_t> ringSizes;

    auto append = [&coords, &ringSizes](const GridPnp::Ring& r)
    {
        ringSizes.push_back(r.size());
        for (const GridPnp::Point& p : r)
        {
            coords.push_back(p.first);
            coords.push_back(p.second);
        }
    };

    append(outer);
    for (const GridPnp::Ring& inner : inners)
        append(inner);

    pdal_gridpnp_t* handle =
        pdal_gridpnp_create(coords.data(), ringSizes.data(), ringSizes.size());
    if (!handle)
    {
        const char* err = pdal_last_error();
        throw grid_error(err && *err ? std::string(err)
                                     : "Failed to build point-in-polygon grid.");
    }
    return handle;
}

} // unnamed namespace

GridPnp::GridPnp(const Ring& outer, const std::vector<Ring>& inners)
    : m_handle(buildHandle(outer, inners))
{
}

GridPnp::GridPnp(const Ring& outer)
    : m_handle(buildHandle(outer, std::vector<Ring>()))
{
}

GridPnp::~GridPnp()
{
    pdal_gridpnp_destroy(m_handle);
}

bool GridPnp::inside(double x, double y) const
{
    return pdal_gridpnp_inside(m_handle, x, y);
}

} // namespace pdal
