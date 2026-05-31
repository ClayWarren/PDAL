/******************************************************************************
 * Copyright (c) 2015, Howard Butler (howard@hobu.co)
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

#include "DensityKernel.hpp"

#include <pdal/util/FileUtils.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

#include <vector>

namespace pdal
{

static PluginInfo const s_info{"kernels.density", "Density Kernel [DEPRECATED]",
                               "https://pdal.org/apps/density.html"};

CREATE_STATIC_KERNEL(DensityKernel, s_info)

std::string DensityKernel::getName() const
{
    return s_info.name;
}

void DensityKernel::addSwitches(ProgramArgs& args)
{
    args.add("input,i", "input point cloud file name", m_inputFile)
        .setPositional();
    args.add("output,o", "output vector data source", m_outputFile)
        .setPositional();
    args.add("ogrdriver,f", "OGR driver name to use ", m_driverName,
             "ESRI Shapefile");
    args.add("lyr_name", "OGR layer name to use", m_layerName, "");
    args.add("sample_size", "Sample size for auto-edge length calculation",
             m_sampleSize, 5000U);
    args.add("threshold", "Required cell density", m_density, 15);
    args.add("edge_length", "Length of hex edge", m_edgeLength);
    args.add("hole_cull_area_tolerance",
             "Tolerance area to "
             "apply to holes before cull",
             m_cullArea);
    args.add("smooth", "Smooth boundary output", m_doSmooth, true);
    args.add("h3_grid",
             "Create a grid using H3 (https://h3geo.org/docs) Hexagons", m_isH3,
             false);
    args.add("h3_resolution",
             "H3 grid resolution: 0 (coarsest) - 15 (finest). See "
             "https://h3geo.org/docs/core-library/restable",
             m_h3Res, -1);
}

int DensityKernel::execute()
{
    if (FileUtils::extension(m_inputFile) == ".xml")
    {
        m_manager.readPipeline(m_inputFile);
        Options options;
        options.add("sample_size", m_sampleSize);
        options.add("threshold", m_density);
        options.add("edge_length", m_edgeLength);
        options.add("hole_cull_area_tolerance", m_cullArea);
        options.add("smooth", m_doSmooth);
        options.add("h3_grid", m_isH3);
        options.add("h3_resolution", m_h3Res);
        options.add("density", m_outputFile);
        options.add("ogrdriver", m_driverName);
        options.add("lyr_name", m_layerName);
        m_manager.makeFilter("filters.hexbin", *m_manager.getStage(), options);
        m_manager.execute();
        return 0;
    }

    StringList args;
    args.push_back(m_inputFile);
    args.push_back(m_outputFile);
    if (!m_driverName.empty())
    {
        args.push_back("--ogrdriver");
        args.push_back(m_driverName);
    }
    if (!m_layerName.empty())
    {
        args.push_back("--lyr_name");
        args.push_back(m_layerName);
    }
    if (m_edgeLength != 0.0)
    {
        args.push_back("--edge_length");
        args.push_back(std::to_string(m_edgeLength));
    }
    args.push_back("--threshold");
    args.push_back(std::to_string(m_density));
    args.push_back("--filters.hexbin.sample_size=" +
                   std::to_string(m_sampleSize));
    args.push_back("--filters.hexbin.smooth=" +
                   std::string(m_doSmooth ? "true" : "false"));
    if (m_cullArea != 0.0)
    {
        args.push_back("--filters.hexbin.hole_cull_area_tolerance=" +
                       std::to_string(m_cullArea));
    }
    if (m_isH3)
    {
        args.push_back("--filters.hexbin.h3_grid=true");
        if (m_h3Res != -1)
            args.push_back("--filters.hexbin.h3_resolution=" +
                           std::to_string(m_h3Res));
    }

    std::vector<const char*> argv;
    argv.reserve(args.size());
    for (const std::string& arg : args)
        argv.push_back(arg.c_str());

    return pdal_rust_kernel_run("density", static_cast<int>(argv.size()),
                                argv.data());
}

} // namespace pdal
