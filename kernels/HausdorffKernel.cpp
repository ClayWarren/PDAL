/******************************************************************************
 * Copyright (c) 2016, Bradley J Chambers (brad.chambers@gmail.com)
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

#include "HausdorffKernel.hpp"

#include <pdal/PDALUtils.hpp>
#include <pdal/pdal_config.hpp>

#include <pdal_capi.h>

namespace pdal
{

static StaticPluginInfo const s_info{"kernels.hausdorff",
                                     "Hausdorff Kernel [DEPRECATED]",
                                     "https://pdal.org/apps/hausdorff.html"};

CREATE_STATIC_KERNEL(HausdorffKernel, s_info)

std::string HausdorffKernel::getName() const
{
    return s_info.name;
}

void HausdorffKernel::addSwitches(ProgramArgs& args)
{
    Arg& source = args.add("source", "Source filename", m_sourceFile);
    source.setPositional();
    Arg& candidate =
        args.add("candidate", "Candidate filename", m_candidateFile);
    candidate.setPositional();
}

int HausdorffKernel::execute()
{
    // The Hausdorff and modified-Hausdorff distances are computed by the Rust
    // metrics engine (pdal-core::metrics::hausdorff_pair) via the C ABI, which
    // loads both clouds through the Rust readers.
    double hausdorff = 0.0;
    double modified = 0.0;
    if (pdal_hausdorff(m_sourceFile.c_str(), m_candidateFile.c_str(),
                       &hausdorff, &modified) != 0)
    {
        const char* message = pdal_last_error();
        throw pdal_error(message ? message
                                 : "Rust hausdorff computation failed.");
    }

    MetadataNode root;
    root.add("filenames", m_sourceFile);
    root.add("filenames", m_candidateFile);
    root.add("hausdorff", hausdorff);
    root.add("modified_hausdorff", modified);
    root.add("pdal_version", Config::fullVersionString());
    Utils::toJSON(root, std::cout);

    return 0;
}

} // namespace pdal
