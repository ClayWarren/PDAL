/******************************************************************************
 * Copyright (c) 2011, Michael P. Gerlek (mpg@flaxen.com)
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

#include <assert.h>
#include <iostream>
#include <limits>
#include <locale>
#include <sstream>
#include <vector>

#include <pdal/util/Bounds.hpp>
#include <pdal/util/Utils.hpp>
#include <pdal_capi.h>

namespace pdal
{

namespace
{

const double LOWEST = (std::numeric_limits<double>::lowest)();
const double HIGHEST = (std::numeric_limits<double>::max)();

pdal_bounds2d_t toRust(const BOX2D& box)
{
    return pdal_bounds2d_t{box.minx, box.maxx, box.miny, box.maxy};
}

pdal_bounds3d_t toRust(const BOX3D& box)
{
    return pdal_bounds3d_t{box.minx, box.maxx, box.miny, box.maxy, box.minz,
                           box.maxz};
}

std::string takeRustString(char* value)
{
    if (!value)
        return std::string();
    std::string out(value);
    pdal_string_free(value);
    return out;
}

} // namespace

bool BOX2D::equal(const BOX2D& other) const
{
    pdal_bounds2d_t left = toRust(*this);
    pdal_bounds2d_t right = toRust(other);
    return pdal_bounds2d_equal(&left, &right);
}

bool BOX3D::equal(const BOX3D& other) const
{
    pdal_bounds3d_t left = toRust(*this);
    pdal_bounds3d_t right = toRust(other);
    return pdal_bounds3d_equal(&left, &right);
}

std::string box2dToString(const BOX2D& bounds, uint32_t precision)
{
    pdal_bounds2d_t rustBounds = toRust(bounds);
    return takeRustString(pdal_bounds2d_format(&rustBounds, precision));
}

std::string box3dToString(const BOX3D& bounds, uint32_t precision)
{
    pdal_bounds3d_t rustBounds = toRust(bounds);
    return takeRustString(pdal_bounds3d_format(&rustBounds, precision));
}

std::string box2dToWkt(const BOX2D& bounds, uint32_t precision)
{
    pdal_bounds2d_t rustBounds = toRust(bounds);
    return takeRustString(pdal_bounds2d_to_wkt(&rustBounds, precision));
}

std::string box3dToWkt(const BOX3D& bounds, uint32_t precision)
{
    pdal_bounds3d_t rustBounds = toRust(bounds);
    return takeRustString(pdal_bounds3d_to_wkt(&rustBounds, precision));
}

std::string box2dToGeoJson(const BOX2D& bounds, uint32_t precision)
{
    pdal_bounds2d_t rustBounds = toRust(bounds);
    return takeRustString(pdal_bounds2d_to_geojson(&rustBounds, precision));
}

std::string BOX2D::toWKT(uint32_t precision) const
{
    return box2dToWkt(*this, precision);
}

std::string BOX2D::toGeoJSON(uint32_t precision) const
{
    return box2dToGeoJson(*this, precision);
}

std::string BOX3D::toWKT(uint32_t precision) const
{
    return box3dToWkt(*this, precision);
}

void BOX2D::clear()
{
    pdal_bounds2d_t bounds;
    pdal_bounds2d_clear(&bounds);
    minx = bounds.minx;
    maxx = bounds.maxx;
    miny = bounds.miny;
    maxy = bounds.maxy;
    wkt = "";
}

void BOX3D::clear()
{
    pdal_bounds3d_t bounds;
    pdal_bounds3d_clear(&bounds);
    minx = bounds.minx;
    maxx = bounds.maxx;
    miny = bounds.miny;
    maxy = bounds.maxy;
    minz = bounds.minz;
    maxz = bounds.maxz;
    wkt = "";
}

bool BOX2D::empty() const
{
    pdal_bounds2d_t bounds{minx, maxx, miny, maxy};
    return pdal_bounds2d_empty(&bounds);
}

bool BOX2D::valid() const
{
    return !empty();
}

bool BOX3D::empty() const
{
    pdal_bounds3d_t bounds{minx, maxx, miny, maxy, minz, maxz};
    return pdal_bounds3d_empty(&bounds);
}

bool BOX3D::valid() const
{
    return !empty();
}

bool BOX2D::contains(double x, double y) const
{
    pdal_bounds2d_t bounds{minx, maxx, miny, maxy};
    return pdal_bounds2d_contains_point(&bounds, x, y);
}

bool BOX2D::contains(const BOX2D& other) const
{
    pdal_bounds2d_t bounds{minx, maxx, miny, maxy};
    pdal_bounds2d_t rustOther{other.minx, other.maxx, other.miny, other.maxy};
    return pdal_bounds2d_contains_bounds(&bounds, &rustOther);
}

bool BOX2D::overlaps(const BOX2D& other) const
{
    pdal_bounds2d_t bounds{minx, maxx, miny, maxy};
    pdal_bounds2d_t rustOther{other.minx, other.maxx, other.miny, other.maxy};
    return pdal_bounds2d_overlaps(&bounds, &rustOther);
}

BOX2D& BOX2D::grow(double dist)
{
    assert(valid());
    pdal_bounds2d_t bounds{minx, maxx, miny, maxy};
    pdal_bounds2d_grow_distance(&bounds, dist);
    minx = bounds.minx;
    maxx = bounds.maxx;
    miny = bounds.miny;
    maxy = bounds.maxy;
    return *this;
}

BOX2D& BOX2D::grow(double x, double y)
{
    pdal_bounds2d_t bounds{minx, maxx, miny, maxy};
    pdal_bounds2d_grow_point(&bounds, x, y);
    minx = bounds.minx;
    maxx = bounds.maxx;
    miny = bounds.miny;
    maxy = bounds.maxy;
    return *this;
}

BOX2D& BOX2D::grow(const BOX2D& other)
{
    pdal_bounds2d_t bounds{minx, maxx, miny, maxy};
    pdal_bounds2d_t rustOther{other.minx, other.maxx, other.miny, other.maxy};
    pdal_bounds2d_grow_bounds(&bounds, &rustOther);
    minx = bounds.minx;
    maxx = bounds.maxx;
    miny = bounds.miny;
    maxy = bounds.maxy;
    return *this;
}

void BOX2D::clip(const BOX2D& other)
{
    pdal_bounds2d_t bounds{minx, maxx, miny, maxy};
    pdal_bounds2d_t rustOther{other.minx, other.maxx, other.miny, other.maxy};
    pdal_bounds2d_clip(&bounds, &rustOther);
    minx = bounds.minx;
    maxx = bounds.maxx;
    miny = bounds.miny;
    maxy = bounds.maxy;
}

BOX3D& BOX3D::grow(double x, double y, double z)
{
    pdal_bounds3d_t bounds{minx, maxx, miny, maxy, minz, maxz};
    pdal_bounds3d_grow_point(&bounds, x, y, z);
    minx = bounds.minx;
    maxx = bounds.maxx;
    miny = bounds.miny;
    maxy = bounds.maxy;
    minz = bounds.minz;
    maxz = bounds.maxz;
    return *this;
}

BOX3D& BOX3D::grow(const BOX3D& other)
{
    pdal_bounds3d_t bounds{minx, maxx, miny, maxy, minz, maxz};
    pdal_bounds3d_t rustOther{other.minx, other.maxx, other.miny,
                              other.maxy, other.minz, other.maxz};
    pdal_bounds3d_grow_bounds(&bounds, &rustOther);
    minx = bounds.minx;
    maxx = bounds.maxx;
    miny = bounds.miny;
    maxy = bounds.maxy;
    minz = bounds.minz;
    maxz = bounds.maxz;
    return *this;
}

BOX3D& BOX3D::grow(double dist)
{
    assert(valid());
    pdal_bounds3d_t bounds{minx, maxx, miny, maxy, minz, maxz};
    pdal_bounds3d_grow_distance(&bounds, dist);
    minx = bounds.minx;
    maxx = bounds.maxx;
    miny = bounds.miny;
    maxy = bounds.maxy;
    minz = bounds.minz;
    maxz = bounds.maxz;
    return *this;
}

void BOX3D::clip(const BOX3D& other)
{
    pdal_bounds3d_t bounds{minx, maxx, miny, maxy, minz, maxz};
    pdal_bounds3d_t rustOther{other.minx, other.maxx, other.miny,
                              other.maxy, other.minz, other.maxz};
    pdal_bounds3d_clip(&bounds, &rustOther);
    minx = bounds.minx;
    maxx = bounds.maxx;
    miny = bounds.miny;
    maxy = bounds.maxy;
    minz = bounds.minz;
    maxz = bounds.maxz;
}

bool BOX3D::contains(double x, double y, double z) const
{
    pdal_bounds3d_t bounds{minx, maxx, miny, maxy, minz, maxz};
    return pdal_bounds3d_contains_point(&bounds, x, y, z);
}

bool BOX3D::contains(const BOX3D& other) const
{
    pdal_bounds3d_t bounds{minx, maxx, miny, maxy, minz, maxz};
    pdal_bounds3d_t rustOther{other.minx, other.maxx, other.miny,
                              other.maxy, other.minz, other.maxz};
    return pdal_bounds3d_contains_bounds(&bounds, &rustOther);
}

bool BOX3D::overlaps(const BOX3D& other) const
{
    pdal_bounds3d_t bounds{minx, maxx, miny, maxy, minz, maxz};
    pdal_bounds3d_t rustOther{other.minx, other.maxx, other.miny,
                              other.maxy, other.minz, other.maxz};
    return pdal_bounds3d_overlaps(&bounds, &rustOther);
}

const BOX2D& BOX2D::getDefaultSpatialExtent()
{
    static BOX2D v;
    static bool initialized = false;
    if (!initialized)
    {
        pdal_bounds2d_t bounds;
        pdal_bounds2d_default(&bounds);
        v.minx = bounds.minx;
        v.maxx = bounds.maxx;
        v.miny = bounds.miny;
        v.maxy = bounds.maxy;
        initialized = true;
    }
    return v;
}

const BOX3D& BOX3D::getDefaultSpatialExtent()
{
    static BOX3D v;
    static bool initialized = false;
    if (!initialized)
    {
        pdal_bounds3d_t bounds;
        pdal_bounds3d_default(&bounds);
        v.minx = bounds.minx;
        v.maxx = bounds.maxx;
        v.miny = bounds.miny;
        v.maxy = bounds.maxy;
        v.minz = bounds.minz;
        v.maxz = bounds.maxz;
        initialized = true;
    }
    return v;
}

Bounds::Bounds(const BOX3D& box) : m_box(box) {}

Bounds::Bounds(const BOX2D& box) : m_box(box)
{
    m_box.minz = HIGHEST;
    m_box.maxz = LOWEST;
}

void Bounds::reset(const BOX3D& box)
{
    m_box = box;
}

void Bounds::reset(const BOX2D& box)
{
    m_box.minx = box.minx;
    m_box.maxx = box.maxx;
    m_box.miny = box.miny;
    m_box.maxy = box.maxy;
    m_box.minz = HIGHEST;
    m_box.maxz = LOWEST;
    m_box.wkt = box.wkt;
}

// We don't allow implicit conversion from a BOX2D to BOX3D.  Use the explicit
// BOX3D ctor that takes a BOX2D if that's what you want.
BOX3D Bounds::to3d() const
{
    if (!is3d())
        return BOX3D();
    return m_box;
}

BOX2D Bounds::to2d() const
{
    return m_box.to2d();
}

bool Bounds::is2d() const
{
    return (valid() && !is3d());
}

bool Bounds::is3d() const
{
    return (m_box.minz != HIGHEST || m_box.maxz != LOWEST);
}

bool Bounds::valid() const
{
    return m_box.valid();
}

bool Bounds::empty() const
{
    return m_box.empty();
}

void Bounds::grow(double x, double y)
{
    if (!is3d())
    {
        m_box.minx = (std::min)(x, m_box.minx);
        m_box.miny = (std::min)(y, m_box.miny);
        m_box.maxx = (std::max)(x, m_box.maxx);
        m_box.maxy = (std::max)(y, m_box.maxy);
    }
}

void Bounds::grow(double x, double y, double z)
{
    if (!is2d())
    {
        m_box.grow(x, y, z);
    }
}

void Bounds::set(const BOX3D& box)
{
    m_box = box;
}

void Bounds::set(const BOX2D& box)
{
    m_box = BOX3D(box);
    m_box.minz = HIGHEST;
    m_box.maxz = LOWEST;
}

// This parses the guts of a 2D range.
void BOX2D::parse(const std::string& s, std::string::size_type& pos)
{
    pdal_bounds2d_t bounds;
    char* rustWkt = nullptr;
    uint64_t rustPos = pos;
    char* parseError =
        pdal_bounds2d_parse(s.c_str(), pos, &bounds, &rustWkt, &rustPos);
    if (parseError)
    {
        std::string message(parseError);
        pdal_string_free(parseError);
        pdal_string_free(rustWkt);
        throw error(message);
    }

    minx = bounds.minx;
    maxx = bounds.maxx;
    miny = bounds.miny;
    maxy = bounds.maxy;
    wkt = rustWkt ? rustWkt : "";
    pos = rustPos;
    pdal_string_free(rustWkt);
}

void BOX3D::parse(const std::string& s, std::string::size_type& pos)
{
    pdal_bounds3d_t bounds;
    char* rustWkt = nullptr;
    uint64_t rustPos = pos;
    char* parseError =
        pdal_bounds3d_parse(s.c_str(), pos, &bounds, &rustWkt, &rustPos);
    if (parseError)
    {
        std::string message(parseError);
        pdal_string_free(parseError);
        pdal_string_free(rustWkt);
        throw error(message);
    }

    minx = bounds.minx;
    maxx = bounds.maxx;
    miny = bounds.miny;
    maxy = bounds.maxy;
    minz = bounds.minz;
    maxz = bounds.maxz;
    wkt = rustWkt ? rustWkt : "";
    pos = rustPos;
    pdal_string_free(rustWkt);
}

std::istream& operator>>(std::istream& in, BOX2D& box)
{
    std::string s;

    std::getline(in, s);
    std::string::size_type pos(0);

    box.parse(s, pos);
    if (pos != s.size())
        throw BOX2D::error("Invalid characters following valid 2d-bounds.");
    return in;
}

std::istream& operator>>(std::istream& in, BOX3D& box)
{
    std::string s;

    std::getline(in, s);
    std::string::size_type pos(0);

    try
    {
        BOX3D box3d;
        box.parse(s, pos);
    }
    catch (const BOX3D::error&)
    {
        try
        {
            pos = 0;
            BOX2D box2d;
            box2d.parse(s, pos);
            box = BOX3D(box2d);
        }
        catch (const BOX2D::error& err)
        {
            throw BOX3D::error(err.what());
        }
    }

    return in;
}

void Bounds::parse(const std::string& s, std::string::size_type& pos)
{
    try
    {
        BOX3D box3d;
        box3d.parse(s, pos);
        set(box3d);
    }
    catch (const BOX3D::error&)
    {
        try
        {
            pos = 0;
            BOX2D box2d;
            box2d.parse(s, pos);
            set(box2d);
        }
        catch (const BOX2D::error& err)
        {
            throw Bounds::error(err.what());
        }
    }
}

std::istream& operator>>(std::istream& in, Bounds& bounds)
{
    std::string s;

    std::getline(in, s);
    std::string::size_type pos(0);

    bounds.parse(s, pos);
    return in;
}

std::ostream& operator<<(std::ostream& out, const Bounds& bounds)
{
    if (bounds.is3d())
        out << bounds.to3d();
    else
        out << bounds.to2d();
    return out;
}

} // namespace pdal
