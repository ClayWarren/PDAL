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

#include "SortKernel.hpp"

#include <rust/pdal-capi/include/pdal_capi.h>

#include <vector>

namespace pdal
{

static StaticPluginInfo const s_info{"kernels.sort", "Sort Kernel",
                                     "https://pdal.org/apps/sort.html"};

CREATE_STATIC_KERNEL(SortKernel, s_info)

std::string SortKernel::getName() const
{
    return s_info.name;
}

SortKernel::SortKernel() : m_bCompress(false), m_bForwardMetadata(false) {}

void SortKernel::addSwitches(ProgramArgs& args)
{
    args.add("input,i", "Input filename", m_inputFile).setPositional();
    args.add("output,o", "Output filename", m_outputFile).setPositional();
    args.add("compress,z",
             "Compress output data (if supported by output format)",
             m_bCompress);
    args.add(
        "metadata,m",
        "Forward metadata (VLRs, header entries, etc) from previous stages",
        m_bForwardMetadata);
}

int SortKernel::execute()
{
    StringList args;
    if (!m_driverOverride.empty())
    {
        args.push_back("--driver");
        args.push_back(m_driverOverride);
    }
    args.push_back(m_inputFile);
    args.push_back(m_outputFile);
    if (m_bCompress)
        args.push_back("--writers.las.compression=true");
    if (m_bForwardMetadata)
        args.push_back("--writers.las.forward_metadata=true");

    std::vector<const char*> argv;
    argv.reserve(args.size());
    for (const std::string& arg : args)
        argv.push_back(arg.c_str());

    return pdal_rust_kernel_run("sort", static_cast<int>(argv.size()),
                                argv.data());
}

} // namespace pdal
