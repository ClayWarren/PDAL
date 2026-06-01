/******************************************************************************
 * Copyright (c) 2020, Hobu Inc. (info@hobu.co)
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

#include <array>

#include <nlohmann/json.hpp>

#include <pdal/private/MathUtils.hpp>
#include <pdal/private/SrsTransform.hpp>
#include <pdal/util/Bounds.hpp>

#include <pdal_capi.h>

#include "Obb.hpp"

namespace pdal
{
namespace i3s
{

Obb::Obb() : m_valid(false) {}

Obb::Obb(const NL::json& spec)
{
    parse(spec);
}

bool Obb::valid() const
{
    return m_valid;
}

Eigen::Vector3d Obb::center() const
{
    return m_p;
}

Eigen::Quaterniond Obb::quat() const
{
    return m_quat;
}

BOX3D Obb::bounds() const
{
    return {-m_hx, -m_hy, -m_hz, m_hx, m_hy, m_hz};
}

void Obb::verifyArray(const NL::json& spec, const std::string& name, size_t cnt)
{
    if (spec.count(name) != 1)
        throw EsriError("Invalid OBB - missing '" + name + "' entry.");

    const NL::json& arr = spec[name];
    if (!arr.is_array())
        throw EsriError("Invalid OBB - '" + name + "' is not an array.");
    if (arr.size() != cnt)
        throw EsriError("Invalid OBB - '" + name +
                        "' does not specify "
                        "three values.");
    for (size_t i = 0; i < cnt; ++i)
    {
        const NL::json& o = arr[i];
        if (!o.is_number())
            throw EsriError("Invalid OBB - '" + name + "[" + std::to_string(i) +
                            "]' "
                            "is not numeric.");
    }
}

void Obb::parse(NL::json spec)
{
    verifyArray(spec, "center", 3);
    verifyArray(spec, "halfSize", 3);
    verifyArray(spec, "quaternion", 4);

    double x = spec["center"][0].get<double>();
    double y = spec["center"][1].get<double>();
    double z = spec["center"][2].get<double>();
    m_p = {x, y, z};

    m_hx = spec["halfSize"][0].get<double>();
    m_hy = spec["halfSize"][1].get<double>();
    m_hz = spec["halfSize"][2].get<double>();

    double qx = spec["quaternion"][0].get<double>();
    double qy = spec["quaternion"][1].get<double>();
    double qz = spec["quaternion"][2].get<double>();
    double qw = spec["quaternion"][3].get<double>();

    m_quat = Eigen::Quaterniond(qw, qx, qy, qz);
    m_quat.normalize();

    spec.erase("center");
    spec.erase("halfSize");
    spec.erase("quaternion");
    if (spec.size())
    {
        throw EsriError("Invalid OBB: found invalid key '" +
                        spec.begin().key() + "'.");
    }
    m_valid = true;
}

void Obb::transform(const SrsTransform& xform)
{
    xform.transform(m_p.x(), m_p.y(), m_p.z());
}

// For this to work both this and the clip box must be in the same cartesian
// system. The intersection geometry is routed through the Rust C ABI.
bool Obb::intersect(Obb c)
{
    const double centerA[3] = {m_p.x(), m_p.y(), m_p.z()};
    const double halfA[3] = {m_hx, m_hy, m_hz};
    const double quatA[4] = {m_quat.x(), m_quat.y(), m_quat.z(), m_quat.w()};
    const double centerB[3] = {c.m_p.x(), c.m_p.y(), c.m_p.z()};
    const double halfB[3] = {c.m_hx, c.m_hy, c.m_hz};
    const double quatB[4] = {c.m_quat.x(), c.m_quat.y(), c.m_quat.z(),
                             c.m_quat.w()};
    return pdal_obb_intersect(centerA, halfA, quatA, centerB, halfB, quatB);
}

void Obb::setCenter(const Eigen::Vector3d& center)
{
    m_p = center;
}

std::ostream& operator<<(std::ostream& out, const Obb& obb)
{
    NL::json j;
    j["center"] = {obb.m_p.x(), obb.m_p.y(), obb.m_p.z()};
    j["halfSize"] = {obb.m_hx, obb.m_hy, obb.m_hz};
    const Eigen::Vector3d& v = obb.m_quat.vec();
    j["quaternion"] = {v.x(), v.y(), v.z(), obb.m_quat.w()};

    out << j;
    return out;
}

} // namespace i3s
} // namespace pdal
