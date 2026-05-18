/******************************************************************************
 * Copyright (c) 2026, PDAL Rust Port Contributors
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
 *     * Neither the name of Hobu, Inc. nor the
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

#include <pdal/Kernel.hpp>
#include <pdal/PluginManager.hpp>
#include <pdal/StageFactory.hpp>
#include <pdal/pdal_config.hpp>
#include <pdal/util/Utils.hpp>

#include <nlohmann/json.hpp>
#include <cstring>
#include <mutex>
#include <sstream>

using namespace pdal;

static std::once_flag s_pluginsLoaded;

static void ensurePluginsLoaded()
{
    std::call_once(s_pluginsLoaded, []()
                   { PluginManager<Kernel>::loadAll();
                   PluginManager<Stage>::loadAll(); });
}

extern "C"
{

    const char* pdal_version_string()
    {
        static const std::string version = Config::fullVersionString();
        return version.c_str();
    }

    char* pdal_kernel_list_json()
    {
        ensurePluginsLoaded();

        NL::json arr = NL::json::array();
        std::string kernelbase("kernels.");
        for (const auto& name : PluginManager<Kernel>::names())
        {
            std::string shortName = name;
            if (Utils::startsWith(name, kernelbase))
                shortName = name.substr(kernelbase.size());

            arr.push_back({{"name", shortName},
                           {"full_name", name},
                           {"description", PluginManager<Kernel>::description(name)}});
        }

        std::string result = arr.dump();
        char* buf = static_cast<char*>(malloc(result.size() + 1));
        if (buf)
            std::memcpy(buf, result.c_str(), result.size() + 1);
        return buf;
    }

    char* pdal_stage_list_json()
    {
        ensurePluginsLoaded();

        NL::json arr = NL::json::array();
        for (const auto& name : PluginManager<Stage>::names())
        {
            arr.push_back({{"name", name},
                           {"description", PluginManager<Stage>::description(name)},
                           {"link", PluginManager<Stage>::link(name)}});
        }

        std::string result = arr.dump();
        char* buf = static_cast<char*>(malloc(result.size() + 1));
        if (buf)
            std::memcpy(buf, result.c_str(), result.size() + 1);
        return buf;
    }

    char* pdal_stage_options_json(const char* stage_name)
    {
        if (!stage_name)
            return nullptr;

        ensurePluginsLoaded();

        StageFactory factory(false);
        Stage* stage = factory.createStage(stage_name);
        if (!stage)
            return nullptr;

        ProgramArgs args;
        stage->addAllArgs(args);

        std::ostringstream ostr;
        args.dump3(ostr);

        std::string result = ostr.str();
        char* buf = static_cast<char*>(malloc(result.size() + 1));
        if (buf)
            std::memcpy(buf, result.c_str(), result.size() + 1);
        return buf;
    }

    int pdal_kernel_run(const char* kernel_name, int argc, const char* const* argv)
    {
        if (!kernel_name || !argv)
            return 1;

        ensurePluginsLoaded();

        std::string fullName = kernel_name;
        if (!Utils::startsWith(fullName, "kernels."))
            fullName = "kernels." + fullName;

        Kernel* kernel = PluginManager<Kernel>::createObject(fullName);
        if (!kernel)
            return 1;

        StringList cmdArgs;
        for (int i = 0; i < argc; ++i)
            cmdArgs.push_back(argv[i]);

        LogPtr log = Log::makeLog("pdal", "stderr");
        int ret = kernel->run(cmdArgs, log);

        delete kernel;
        return ret;
    }

    void pdal_capi_free(void* ptr)
    {
        if (ptr)
            free(ptr);
    }

} // extern "C"
