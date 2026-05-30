/******************************************************************************
 * Copyright (c) 2018, Hobu Inc. (info@hobu.co)
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

#include "TileKernel.hpp"

#include <rust/pdal-capi/include/pdal_capi.h>

#include <pdal/Writer.hpp>

#include <cmath>
#include <vector>

namespace pdal
{

static StaticPluginInfo const s_info{"kernels.tile", "Tile Kernel",
                                     "https://pdal.org/apps/tile.html"};

CREATE_STATIC_KERNEL(TileKernel, s_info)

TileKernel::TileKernel() : m_table(10000), m_repro(nullptr) {}

std::string TileKernel::getName() const
{
    return s_info.name;
}

void TileKernel::addSwitches(ProgramArgs& args)
{
    args.add("input,i", "Input file/path name", m_inputFile).setPositional();
    args.add("output,o", "Output filename template", m_outputFile)
        .setPositional();
    args.add("length", "Edge length for cells", m_length, 1000.0);
    args.add("origin_x", "Origin in X axis for cells", m_xOrigin,
             std::numeric_limits<double>::quiet_NaN());
    args.add("origin_y", "Origin in Y axis for cells", m_yOrigin,
             std::numeric_limits<double>::quiet_NaN());
    args.add("buffer", "Size of buffer (overlap) to include around each tile",
             m_buffer);
    args.add("out_srs", "Output SRS to which points will be reprojected",
             m_outSrs);
}

void TileKernel::validateSwitches(ProgramArgs& args)
{
    m_outputFile = Writer::replaceTags(m_outputFile);
    m_hashPos = Writer::handleFilenameTemplate(m_outputFile);
    if (m_hashPos == std::string::npos)
        throw pdal_error("Output filename must contain a single '#' "
                         "template placeholder.");
}

int TileKernel::execute()
{
    StringList args;
    args.push_back(m_inputFile);
    args.push_back(m_outputFile);
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
    args.push_back("--buffer");
    args.push_back(std::to_string(m_buffer));
    if (!m_outSrs.empty())
    {
        args.push_back("--out_srs");
        args.push_back(m_outSrs.getWKT());
    }

    std::vector<const char*> argv;
    argv.reserve(args.size());
    for (const std::string& arg : args)
        argv.push_back(arg.c_str());

    return pdal_rust_kernel_run("tile", static_cast<int>(argv.size()),
                                argv.data());
}

} // namespace pdal
