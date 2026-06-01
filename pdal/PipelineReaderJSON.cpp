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

#include <nlohmann/json.hpp>

#include <pdal/Filter.hpp>
#include <pdal/Options.hpp>
#include <pdal/PipelineManager.hpp>
#include <pdal/PipelineReaderJSON.hpp>
#include <pdal/PluginManager.hpp>
#include <pdal/private/FileSpecHelper.hpp>
#include <pdal/util/Algorithm.hpp>
#include <pdal/util/FileUtils.hpp>
#include <pdal/util/Utils.hpp>

#include <pdal_capi.h>

#include <memory>
#include <vector>

namespace pdal
{

using TagMap = std::map<std::string, Stage*>;

namespace
{

bool extractOption(Options& options, const std::string& name,
                   const NL::json& node);

// Build a stage's Options from the Rust-validated `options` object. The
// `plugin` key is handled here (it loads a plugin and is not stored as an
// option) because plugin loading owns C++ process state.
Options optionsFromJson(const NL::json& node)
{
    Options options;

    for (auto& it : node.items())
    {
        const NL::json& subnode = it.value();
        const std::string& name = it.key();

        if (name == "plugin")
        {
            PluginManager<Stage>::loadPlugin(subnode.get<std::string>());

            // Don't actually put a "plugin" option on
            // any stage
            continue;
        }

        if (subnode.is_array())
        {
            for (const NL::json& val : subnode)
                if (val.is_object())
                    options.add(name, val);
                else if (!extractOption(options, name, val))
                    throw pdal_error("JSON pipeline: Invalid value type for "
                                     "option list '" +
                                     name + "'.");
        }
        else if (subnode.is_object())
            options.add(name, subnode);
        else if (!extractOption(options, name, subnode))
            throw pdal_error("JSON pipeline: Value of stage option '" + name +
                             "' cannot be converted.");
    }
    return options;
}

bool extractOption(Options& options, const std::string& name,
                   const NL::json& node)
{
    if (node.is_string())
        options.add(name, node.get<std::string>());
    else if (node.is_number_unsigned())
        options.add(name, node.get<uint64_t>());
    else if (node.is_number_integer())
        options.add(name, node.get<int64_t>());
    else if (node.is_number_float())
        options.add(name, node.get<double>());
    else if (node.is_boolean())
        options.add(name, node.get<bool>());
    else if (node.is_array())
        options.add(name, node.get<NL::json::array_t>());
    else if (node.is_null())
        options.add(name, "");
    else
        return false;
    return true;
}

// Build a FileSpec from a descriptor. Bare-string pipeline elements are
// ingested directly; object stages parse their `filename` node (which may be a
// string or an object) through FileSpecHelper.
FileSpec extractFilespec(const NL::json& desc)
{
    FileSpec spec;

    if (desc.value("string_node", false))
    {
        spec.ingest(desc.at("filename").get<std::string>());
        return spec;
    }

    const NL::json& filename = desc.at("filename");
    if (filename.is_null())
        return spec;

    NL::json fnode = filename;
    Utils::StatusWithReason status = FileSpecHelper::parse(spec, fnode);
    if (!status)
        throw pdal_error(status.what());
    return spec;
}

// Build the C++ Stage* DAG from the Rust-validated descriptor array. Rust owns
// JSON parsing, comment stripping, root/type/tag/inputs validation, and
// reader/writer/filter role classification; this loop owns FileSpec/Options
// construction, glob expansion, stage creation, and input wiring.
void buildPipeline(const NL::json& descriptors, PipelineManager& manager)
{
    TagMap tags;
    std::vector<Stage*> inputs;

    for (const NL::json& desc : descriptors)
    {
        const std::string role = desc.at("role").get<std::string>();
        const std::string type = desc.at("type").get<std::string>();
        const std::string tag = desc.at("tag").get<std::string>();

        FileSpec spec = extractFilespec(desc);
        Options options = optionsFromJson(desc.at("options"));

        std::vector<Stage*> specifiedInputs;
        for (const NL::json& name : desc.at("inputs"))
            specifiedInputs.push_back(tags.at(name.get<std::string>()));
        if (!specifiedInputs.empty())
            inputs = specifiedInputs;

        Stage* s = nullptr;

        if (role == "reader")
        {
            StringList files = Utils::glob(spec.u8string());
            if (files.empty())
                files.push_back(spec.u8string());

            for (const std::string& path : files)
            {
                spec.setFilePath(path);
                ReaderCreationOptions ops{spec, type, nullptr, options, tag};
                s = &manager.makeReader(ops);
                inputs.push_back(s);
            }
        }
        else if (role == "writer")
        {
            StageCreationOptions ops{spec.u8string(), type, nullptr, options,
                                     tag};
            s = &manager.makeWriter(ops);
            for (Stage* ts : inputs)
                s->setInput(*ts);
            inputs.clear();
            inputs.push_back(s);
        }
        else
        {
            if (spec.valid())
                options.add("filename", spec.u8string());
            StageCreationOptions ops{"", type, nullptr, options, tag};
            s = &manager.makeFilter(ops);
            for (Stage* ts : inputs)
                s->setInput(*ts);
            inputs.clear();
            inputs.push_back(s);
        }
        // 's' should be valid at this point.  makeXXX will throw if the stage
        // couldn't be constructed.
        if (tag.size())
            tags[tag] = s;
    }

    // Tell user if the pipeline seems wacky.
    const std::vector<Stage*> llist = manager.leaves();
    if (llist.size() > 1)
    {
        const LogPtr& log = manager.log();
        log->get(LogLevel::Error) << "Pipeline has multiple leaf nodes.\n";
        log->get(LogLevel::Error)
            << "Only the first of the following leaf nodes will be run.\n";
        for (Stage* s : llist)
        {
            std::string name = s->tag().size() ? s->tag() : s->getName();
            log->get(LogLevel::Error) << "    " << name << "\n";
        }
    }
}

} // unnamed namespace

PipelineReaderJSON::PipelineReaderJSON(PipelineManager& manager)
    : m_manager(manager)
{
}

void PipelineReaderJSON::readPipeline(const std::string& filename)
{
    std::istream* input = Utils::openFile(filename);
    if (!input)
    {
        throw pdal_error("Pipeline: Unable to open stream for "
                         "file \"" +
                         filename + "\"");
    }

    try
    {
        readPipeline(*input);
    }
    catch (...)
    {
        Utils::closeFile(input);
        throw;
    }

    Utils::closeFile(input);
}

void PipelineReaderJSON::readPipeline(std::istream& input)
{
    std::istreambuf_iterator<char> eos;
    std::string json(std::istreambuf_iterator<char>(input), eos);

    // Rust owns the parse + validation contract; it returns a flat,
    // pre-validated descriptor array (or an error message via pdal_last_error).
    char* out = pdal_pipeline_reader_parse_json(json.c_str());
    if (!out)
    {
        const char* err = pdal_last_error();
        throw pdal_error(err ? err : "Failed to parse pipeline JSON.");
    }

    NL::json descriptors;
    try
    {
        descriptors = NL::json::parse(out);
    }
    catch (...)
    {
        pdal_string_free(out);
        throw;
    }
    pdal_string_free(out);

    buildPipeline(descriptors, m_manager);
}

} // namespace pdal
