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

#include "SplitKernel.hpp"

#include <pdal/util/Utils.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

#include <cmath>
#include <vector>

namespace pdal
{

static StaticPluginInfo const s_info{"kernels.split", "Split Kernel",
                                     "https://pdal.org/apps/split.html"};

CREATE_STATIC_KERNEL(SplitKernel, s_info)

std::string SplitKernel::getName() const
{
    return s_info.name;
}

void SplitKernel::addSwitches(ProgramArgs& args)
{
    args.add("input,i", "Input filename", m_inputFile).setPositional();
    args.add("output,o", "Output filename", m_outputFile).setPositional();
    args.add("length", "Edge length for splitter cells", m_length, 0.0);
    args.add("capacity", "Point capacity of chipper cells", m_capacity);
    args.add("origin_x", "Origin in X axis for splitter cells", m_xOrigin,
             std::numeric_limits<double>::quiet_NaN());
    args.add("origin_y", "Origin in Y axis for splitter cells", m_yOrigin,
             std::numeric_limits<double>::quiet_NaN());
}

void SplitKernel::validateSwitches(ProgramArgs& args)
{
    if (m_length && m_capacity)
        throw pdal_error("Can't specify both length and capacity.");
    if (!m_length && !m_capacity)
        m_capacity = 100000;
    if (m_outputFile.back() == Utils::dirSeparator)
        m_outputFile += m_inputFile;
}

int SplitKernel::execute()
{
    StringList args;
    if (!m_driverOverride.empty())
    {
        args.push_back("--driver");
        args.push_back(m_driverOverride);
    }
    args.push_back(m_inputFile);
    args.push_back(m_outputFile);
    if (m_length)
    {
        args.push_back("--length");
        args.push_back(std::to_string(m_length));
        if (!std::isnan(m_xOrigin))
        {
            args.push_back("--origin_x");
            args.push_back(std::to_string(m_xOrigin));
        }
        if (!std::isnan(m_yOrigin))
        {
            args.push_back("--origin_y");
            args.push_back(std::to_string(m_yOrigin));
        }
    }
    else
    {
        args.push_back("--capacity");
        args.push_back(std::to_string(m_capacity));
    }

    std::vector<const char*> argv;
    argv.reserve(args.size());
    for (const std::string& arg : args)
        argv.push_back(arg.c_str());

    return pdal_rust_kernel_run("split", static_cast<int>(argv.size()),
                                argv.data());
}

} // namespace pdal
