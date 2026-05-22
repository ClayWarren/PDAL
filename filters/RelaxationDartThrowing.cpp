/******************************************************************************
 * Copyright (c) 2020, Bradley J Chambers (brad.chambers@gmail.com)
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

#include "RelaxationDartThrowing.hpp"

#include <pdal/private/RustViewConverter.hpp>
#include <pdal/util/ProgramArgs.hpp>

#include <pdal_capi.h>

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.relaxationdartthrowing", "Subsampling filter",
    "https://pdal.org/stages/filters.relaxationdartthrowing.html"};

CREATE_STATIC_STAGE(RelaxationDartThrowing, s_info)

RelaxationDartThrowing::~RelaxationDartThrowing()
{
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);
}

std::string RelaxationDartThrowing::getName() const
{
    return s_info.name;
}

void RelaxationDartThrowing::addArgs(ProgramArgs& args)
{
    args.add("decay", "Decay rate", m_decay, 0.9);
    args.add("radius", "Minimum radius (initial)", m_startRadius, 1.0);
    args.add("terminal_radius", "Minimum radius (terminal)", m_terminalRadius,
             0.001);
    args.add("count", "Target number of points after sampling", m_maxSize,
             (point_count_t)1000);
    args.add("shuffle", "Shuffle points prior to sampling?", m_shuffle, true);
    m_seedArg = &args.add("seed", "Random number generator seed", m_seed);
}

void RelaxationDartThrowing::initialize()
{
    // Subsampling is performed through the Rust C ABI.
    if (m_rust_stage)
        pdal_stage_destroy(m_rust_stage);

    m_rust_stage = pdal_stage_create_relaxationdartthrowing(
        m_decay, m_startRadius, m_terminalRadius, m_maxSize, m_shuffle,
        m_seedArg->set(), m_seed);
    if (!m_rust_stage)
        throwError("Failed to create Rust relaxationdartthrowing stage.");
}

PointViewSet RelaxationDartThrowing::run(PointViewPtr inView)
{
    PointViewSet viewSet;
    viewSet.insert(rust_view_converter::runSingle(m_rust_stage, inView));
    return viewSet;
}

} // namespace pdal
