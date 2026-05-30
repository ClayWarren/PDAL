/******************************************************************************
 * Copyright (c) 2023, Guilhem Villemin (guilhem.villemin@altametris.com)
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

#include "Trajectory.hpp"

#include <pdal/pdal_types.hpp>

#include <pdal_capi.h>

namespace pdal
{
namespace georeference
{

Trajectory::Trajectory(const std::string& filename, const NL::json& opts)
    : m_handle(nullptr)
{
    // Forward the optional `trajectory_options` object (driver `type` plus
    // reader options) to the Rust loader as JSON text.
    std::string optionsJson = opts.is_null() ? std::string("{}") : opts.dump();
    m_handle = pdal_trajectory_create(filename.c_str(), optionsJson.c_str());
    if (!m_handle)
    {
        const char* err = pdal_last_error();
        throw pdal_error(err && *err
                             ? std::string(err)
                             : ("Cannot load trajectory: " + filename));
    }
}

Trajectory::~Trajectory()
{
    pdal_trajectory_destroy(m_handle);
}

bool Trajectory::getTrajPoint(double time, TrajPoint& output) const
{
    return pdal_trajectory_get_point(m_handle, time, &output.roll,
                                     &output.pitch, &output.azimuth,
                                     &output.wanderAngle, &output.x, &output.y,
                                     &output.z, &output.time);
}

} // namespace georeference
} // namespace pdal
