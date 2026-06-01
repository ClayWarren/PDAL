/******************************************************************************
 * Copyright (c) 2014, Brad Chambers (brad.chambers@gmail.com)
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

#include "RandomKernel.hpp"

#include <pdal_capi.h>

#include <sstream>
#include <vector>

namespace pdal
{

static StaticPluginInfo const s_info{"kernels.random",
                                     "Random Kernel [DEPRECATED]",
                                     "https://pdal.org/apps/random.html"};

CREATE_STATIC_KERNEL(RandomKernel, s_info)

std::string RandomKernel::getName() const
{
    return s_info.name;
}

RandomKernel::RandomKernel()
    : m_bCompress(false), m_numPointsToWrite(0), m_distribution("uniform")
{
}

void RandomKernel::addSwitches(ProgramArgs& args)
{
    args.add("output,o", "Output file name", m_outputFile).setPositional();
    args.add("compress,z",
             "Compress output data (if supported by output format)",
             m_bCompress);
    args.add("count", "How many points should we write?", m_numPointsToWrite);
    args.add("bounds", "Extent (in XYZ to clip output to)", m_bounds);
    args.add(
        "mean",
        "A comma-separated or quoted, space-separated list "
        "of means (normal mode): \n--mean 0.0,0.0,0.0\n--mean \"0.0 0.0 0.0\"",
        m_means);
    args.add("stdev",
             "A comma-separated or quoted, space-separated list "
             "of standard deviations (normal mode): \n"
             "--stdev 0.0,0.0,0.0\n--stdev \"0.0 0.0 0.0\"",
             m_stdevs);
    args.add("distribution", "Distribution (uniform / normal)", m_distribution,
             "uniform");
}

int RandomKernel::execute()
{
    StringList args;
    args.push_back(m_outputFile);
    if (m_bCompress)
        args.push_back("--compress");
    args.push_back("--count");
    args.push_back(std::to_string(m_numPointsToWrite));
    if (!m_bounds.empty())
    {
        std::ostringstream out;
        out << m_bounds;
        args.push_back("--bounds");
        args.push_back(out.str());
    }
    args.push_back("--distribution");
    args.push_back(m_distribution);
    if (!m_means.empty())
    {
        args.push_back("--mean");
        args.push_back(m_means);
    }
    if (!m_stdevs.empty())
    {
        args.push_back("--stdev");
        args.push_back(m_stdevs);
    }

    std::vector<const char*> argv;
    argv.reserve(args.size());
    for (const std::string& arg : args)
        argv.push_back(arg.c_str());

    return pdal_rust_kernel_run("random", static_cast<int>(argv.size()),
                                argv.data());
}

} // namespace pdal
