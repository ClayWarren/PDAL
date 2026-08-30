/******************************************************************************
 * Copyright (c) 2013, Howard Butler (hobu.inc@gmail.com)
 * Copyright (c) 2014-2015, Bradley J Chambers (brad.chambers@gmail.com)
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

#include <pdal/Log.hpp>
#include <pdal/PDALUtils.hpp>
#include <pdal/util/Backtrace.hpp>
#include <pdal/util/FileUtils.hpp>
#include <pdal/util/ProgramArgs.hpp>
#include <pdal/util/Utils.hpp>

#include <pdal_capi.h>

#include <iomanip>
#include <iostream>
#include <memory>
#include <sstream>
#include <string>
#include <vector>

#include <nlohmann/json.hpp>

#ifndef _WIN32
#include <csignal>
#include <unistd.h>
#endif

using namespace pdal;

std::string headline(Utils::screenWidth(), '-');

namespace
{
std::string takeCapiString(char* ptr)
{
    if (!ptr)
        return "";
    std::string value(ptr);
    pdal_string_free(ptr);
    return value;
}

NL::json parseCapiJson(char* ptr)
{
    std::string text = takeCapiString(ptr);
    if (text.empty())
        return NL::json();

    try
    {
        return NL::json::parse(text);
    }
    catch (NL::json::parse_error&)
    {
        return NL::json();
    }
}

std::string stageField(const NL::json& stage, const std::string& key)
{
    if (stage.contains(key) && stage[key].is_string())
        return stage[key].get<std::string>();
    return "";
}
} // namespace

class App
{
public:
    App()
        : m_out(std::cout), m_debug(false), m_logLevel(LogLevel::Error),
          m_showDrivers(false), m_help(false), m_showCommands(false),
          m_showVersion(false), m_showJSON(false), m_logtiming(false)
    {
    }

    int execute(StringList& cmdArgs, LogPtr& log);

private:
    void outputVersion();
    void outputHelp(const ProgramArgs& args);
    void outputDrivers();
    void outputCommands(const std::string& leader);
    void outputOptions();
    void outputOptions(const std::string& stageName, std::ostream& strm);
    void addArgs(ProgramArgs& args);

    std::ostream& m_out;

    std::string m_command;
    bool m_debug;
    LogLevel m_logLevel;
    bool m_showDrivers;
    bool m_help;
    bool m_showCommands;
    bool m_showVersion;
    std::string m_showOptions;
    bool m_showJSON;
    std::string m_log;
    bool m_logtiming;
};

void App::outputVersion()
{
    m_out << headline << '\n';
    m_out << "pdal " << pdal_version_string() << '\n';
    m_out << headline << '\n';
    m_out << '\n';
}

void App::outputHelp(const ProgramArgs& args)
{
    m_out << "Usage:" << '\n';
    m_out << "  pdal <options>" << '\n';
    m_out << "  pdal <command> <command options>" << '\n';

    args.dump(m_out, 2, 80);
    m_out << '\n';

    m_out << "The following commands are available:" << '\n';

    outputCommands("  - ");
    m_out << '\n';
    m_out << "See https://pdal.org/apps/ for more detail" << '\n';
}

void App::outputDrivers()
{
    NL::json stages = parseCapiJson(pdal_stage_list_json());
    if (!stages.is_array())
        stages = NL::json::array();

    if (!m_showJSON)
    {
        int nameColLen(28);
        int descripColLen(Utils::screenWidth() - nameColLen - 1);

        std::string tablehead(std::string(nameColLen, '=') + ' ' +
                              std::string(descripColLen, '='));

        m_out << '\n';
        m_out << tablehead << '\n';
        m_out << std::left << std::setw(nameColLen) << "Name" << " Description"
              << '\n';
        m_out << tablehead << '\n';

        m_out << std::left;

        for (auto const& stage : stages)
        {
            std::string name = stageField(stage, "name");
            std::string descrip = stageField(stage, "description");
            StringList lines = Utils::wordWrap(descrip, descripColLen - 1);
            for (size_t i = 0; i < lines.size(); ++i)
            {
                m_out << std::setw(nameColLen) << name << " " << lines[i]
                      << '\n';
                name.clear();
            }
        }

        m_out << tablehead << '\n' << '\n';
    }
    else
    {
        m_out << std::setw(4) << stages;
    }
}

void App::outputCommands(const std::string& leader)
{
    NL::json kernels = parseCapiJson(pdal_kernel_list_json());
    if (!kernels.is_array())
        return;

    if (m_showJSON)
    {
        m_out << std::setw(4) << kernels;
        return;
    }

    for (auto const& kernel : kernels)
    {
        std::string name = stageField(kernel, "name");
        m_out << leader << name << '\n';
    }
}

void App::outputOptions(std::string const& stageName, std::ostream& strm)
{
    if (!m_showJSON)
    {
        std::string text =
            takeCapiString(pdal_stage_options_text(stageName.c_str()));
        if (text.empty())
            std::cerr << "Unable to create stage " << stageName << "\n";
        else
            strm << text;
        return;
    }

    std::string text =
        takeCapiString(pdal_stage_options_json(stageName.c_str()));
    if (text.empty())
    {
        std::cerr << "Unable to create stage " << stageName << "\n";
        return;
    }

    NL::json array;
    try
    {
        array = NL::json::parse(text);
    }
    catch (NL::json::parse_error&)
    {
    }

    NL::json object = {stageName, array};
    strm << object;
}

void App::outputOptions()
{
    NL::json stages = parseCapiJson(pdal_stage_list_json());
    if (!stages.is_array())
        stages = NL::json::array();

    if (!m_showJSON)
    {
        for (auto const& stage : stages)
        {
            outputOptions(stageField(stage, "name"), m_out);
            m_out << '\n';
        }
    }
    else
    {
        std::stringstream strm;
        NL::json options;
        for (auto const& stage : stages)
        {
            outputOptions(stageField(stage, "name"), strm);
            NL::json j;
            try
            {
                strm >> j;
            }
            catch (NL::json::parse_error&)
            {
            }
            options.push_back(j);
            strm.str("");
        }
        m_out << options;
    }
}

void App::addArgs(ProgramArgs& args)
{
    args.add("command", "The PDAL command", m_command).setPositional();
    args.add("debug", "Sets the output level to 3 (option deprecated)",
             m_debug);
    args.add("verbose,v", "Sets the output level (0-8)", m_logLevel,
             LogLevel::None);
    args.add("drivers", "List available drivers", m_showDrivers);
    args.add("help,h", "Display help text", m_help);
    args.add("list-commands", "List available commands", m_showCommands);
    args.add("version", "Show program version", m_showVersion);
    args.add("options", "Show options for specified driver (or 'all')",
             m_showOptions);
    args.add("log",
             "Log filename (accepts stderr, stdout, stdlog, devnull"
             " as special cases)",
             m_log, "stderr");
    args.add("logtiming", "Turn on timing for log messages", m_logtiming);
    Arg& json = args.add("showjson", "List options or drivers as JSON output",
                         m_showJSON);
    json.setHidden();
}

namespace
{
LogPtr logPtr(Log::makeLog("PDAL", "stderr"));
}

#ifdef PDAL_WIN32_STL
int wmain(int argc, wchar_t* argv[], wchar_t* envp[])
#else
int main(int argc, char* argv[])
#endif
{
    App pdal;

    StringList cmdArgs;
    for (int i = 1; i < argc; ++i)
        cmdArgs.push_back(pdal::FileUtils::fromNative(argv[i]));
    return pdal.execute(cmdArgs, logPtr);
}

int App::execute(StringList& cmdArgs, LogPtr& log)
{
    ProgramArgs args;

    addArgs(args);
    try
    {
        args.parseSimple(cmdArgs);
    }
    catch (arg_val_error const& e)
    {
        Utils::printError(e.what());
        return 1;
    }

    log = Log::makeLog("PDAL", m_log, m_logtiming);
    if (m_logLevel != LogLevel::None)
        log->setLevel(m_logLevel);
    else if (m_debug)
        log->setLevel(LogLevel::Debug);
    log->get(LogLevel::Debug) << "Debugging..." << '\n';
#ifndef _WIN32
    if (m_debug)
    {
        signal(SIGSEGV,
               [](int sig)
               {
                   logPtr->get(LogLevel::Debug)
                       << "Segmentation fault (signal 11)\n";
                   StringList lines = Utils::backtrace();

                   for (const auto& l : lines)
                       logPtr->get(LogLevel::Debug) << l << '\n';
                   exit(1);
               });
    }
#endif

    m_command = Utils::tolower(m_command);
    if (!m_command.empty())
    {
        if (m_help)
            cmdArgs.push_back("--help");

        std::vector<const char*> argv;
        argv.reserve(cmdArgs.size());
        for (auto const& arg : cmdArgs)
            argv.push_back(arg.c_str());

        LogLevel kernelLogLevel =
            m_logLevel != LogLevel::None ? m_logLevel : log->getLevel();
        int ret = pdal_kernel_run(
            m_command.c_str(), static_cast<int>(argv.size()), argv.data(),
            m_log.c_str(), static_cast<int>(kernelLogLevel), m_logtiming);
        if (ret != 0)
        {
            NL::json kernels = parseCapiJson(pdal_kernel_list_json());
            bool found = false;
            if (kernels.is_array())
            {
                for (auto const& kernel : kernels)
                    found = found || stageField(kernel, "name") == m_command;
            }
            if (!found)
            {
                std::string msg = takeCapiString(
                    pdal_app_unknown_command_message(m_command.c_str()));
                log->get(LogLevel::Error) << msg << '\n' << '\n';
            }
        }
        return ret;
    }

    if (m_showVersion)
    {
        outputVersion();
        return 0;
    }
    else if (m_showDrivers)
    {
        outputDrivers();
        return 0;
    }
    else if (m_showCommands)
    {
        outputCommands("");
        return 0;
    }
    else if (m_showOptions.size())
    {
        if (m_showOptions == "all")
            outputOptions();
        else
            outputOptions(m_showOptions, m_out);
        return 0;
    }
    else if (m_help)
    {
        outputHelp(args);
        return 0;
    }

    // If we get here, all arguments should be consumed, if not, it's
    // an error.
    if (cmdArgs.size())
    {
        Utils::printError("Unexpected argument '" + cmdArgs[0] + "'.");
        return 1;
    }

    if (!m_help)
        outputHelp(args);
    return 0;
}
