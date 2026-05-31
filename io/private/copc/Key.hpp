/******************************************************************************
 * Copyright (c) 2018, Connor Manning
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

#pragma once

#include <functional> // for hash

#include <pdal/pdal_types.hpp>

#include <pdal/util/Bounds.hpp>
#include <pdal/util/Extractor.hpp>
#include <pdal_capi.h>

namespace pdal
{
namespace copc
{

class PDAL_EXPORT Key
{
    // A depth/X/Y/Z key representing a data node, as well as the bounds of the
    // contained data.
public:
    Key() {}

    Key(const std::string& s)
    {
        fill(s);
    }

    bool valid() const
    {
        return d != -1;
    }

    bool fill(const std::string& s)
    {
        d = -1;
        pdal_copc_key_t key;
        if (!pdal_copc_key_parse(s.c_str(), &key))
            return false;
        d = key.d;
        x = key.x;
        y = key.y;
        z = key.z;
        return true;
    }

    int32_t d = 0;
    int32_t x = 0;
    int32_t y = 0;
    int32_t z = 0;

    std::string toString() const
    {
        return (std::string)(*this);
    }

    operator std::string() const
    {
        pdal_copc_key_t key{d, x, y, z};
        char* raw = pdal_copc_key_to_string(&key);
        std::string out(raw ? raw : "");
        pdal_string_free(raw);
        return out;
    }

    Key child(int32_t dir) const
    {
        pdal_copc_key_t key{d, x, y, z};
        pdal_copc_key_t child;
        if (!pdal_copc_key_child(&key, dir, &child))
            return Key::invalid();
        return Key(child.d, child.x, child.y, child.z);
    }

    BOX3D bounds(const BOX3D& root) const
    {
        pdal_copc_key_t key{d, x, y, z};
        pdal_copc_bounds3d_t in{root.minx, root.maxx, root.miny,
                                root.maxy, root.minz, root.maxz};
        pdal_copc_bounds3d_t out;
        if (!pdal_copc_key_bounds(&key, &in, &out))
            return BOX3D();
        return BOX3D(out.minx, out.miny, out.minz, out.maxx, out.maxy,
                     out.maxz);
    }

    static Key invalid()
    {
        static Key badkey = Key(-1, 0, 0, 0);

        return badkey;
    }

    Key(int d, int x, int y, int z) : d(d), x(x), y(y), z(z) {}
};

inline LeExtractor& operator>>(LeExtractor& in, Key& k)
{
    in >> k.d >> k.x >> k.y >> k.z;
    return in;
}

inline bool operator==(const Key& a, const Key& b)
{
    return a.d == b.d && a.x == b.x && a.y == b.y && a.z == b.z;
}

inline bool operator!=(const Key& a, const Key& b)
{
    return !(a == b);
}

inline bool operator<(const Key& a, const Key& b)
{
    if (a.d < b.d)
        return true;
    if (a.d > b.d)
        return false;

    if (a.x < b.x)
        return true;
    if (a.x > b.x)
        return false;

    if (a.y < b.y)
        return true;
    if (a.y > b.y)
        return false;

    if (a.z < b.z)
        return true;
    return false;
}

inline std::ostream& operator<<(std::ostream& out, const Key& k)
{
    out << k.toString();
    return out;
}

} // namespace copc
} // namespace pdal

namespace std
{
template <> struct hash<pdal::copc::Key>
{
    std::size_t operator()(pdal::copc::Key const& k) const noexcept
    {
        pdal_copc_key_t key{k.d, k.x, k.y, k.z};
        return pdal_copc_key_hash(&key);
    }
};
} // namespace std
