/******************************************************************************
 * Copyright (c) 2025, Hobu Inc.
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

#include "FileSpec.hpp"

#include <nlohmann/json.hpp>

#include <pdal/PDALUtils.hpp>
#include <pdal/private/FileSpecHelper.hpp>
#include <pdal/util/private/JsonSupport.hpp>
#include <pdal_capi.h>

namespace pdal
{

struct FileSpec::Private
{
    std::filesystem::path m_path;
    StringMap m_headers;
    StringMap m_query;

    Utils::StatusWithReason parse(NL::json& node);
    void setFilePath(const std::string& u8path);
};

FileSpec::FileSpec() : m_p(new Private) {}

FileSpec::FileSpec(const std::string& pathOrJson) : m_p(new Private)
{
    (void)ingest(pathOrJson);
}

FileSpec::~FileSpec() {}

FileSpec::FileSpec(const FileSpec& other) : m_p(new Private)
{
    *m_p = *other.m_p;
}

FileSpec& FileSpec::operator=(const FileSpec& other)
{
    *m_p = *other.m_p;
    return *this;
}

FileSpec::FileSpec(FileSpec&& other)
{
    m_p = std::move(other.m_p);
}

FileSpec& FileSpec::operator=(FileSpec&& other)
{
    m_p = std::move(other.m_p);
    return *this;
}

bool FileSpec::valid() const
{
    return !m_p->m_path.empty();
}

bool FileSpec::onlyFilename() const
{
    return m_p->m_headers.empty() && m_p->m_query.empty();
}

std::string FileSpec::u8string() const
{
    return m_p->m_path.u8string();
}

std::filesystem::path FileSpec::filePath() const
{
    return m_p->m_path;
}

void FileSpec::setFilePath(const std::string& u8path)
{
    m_p->setFilePath(u8path);
}

void FileSpec::setFilePath(const std::filesystem::path& path)
{
    m_p->m_path = path;
}

StringMap FileSpec::query() const
{
    return m_p->m_query;
}

StringMap FileSpec::headers() const
{
    return m_p->m_headers;
}

Utils::StatusWithReason FileSpec::ingest(const std::string& pathOrJson)
{
    NL::json json;
    if (Utils::isJSON(pathOrJson))
    {
        auto status = Utils::parseJson(pathOrJson, json);
        if (!status)
            return status;
    }
    // assuming input is a filename
    else
        json = NL::json(pathOrJson);

    return m_p->parse(json);
}

void FileSpec::Private::setFilePath(const std::string& u8path)
{
#ifdef __cpp_lib_char8_t // C++20
    char8_t* pU8path = reinterpret_cast<const char8_t*>(u8path.data());
    m_path = std::filesystem::path(std::u8string_view(pU8path, u8path.size()));
#else // C++17
    m_path = std::filesystem::u8path(u8path);
#endif
}

Utils::StatusWithReason FileSpec::Private::parse(NL::json& node)
{
    const bool wasObject = node.is_object();
    std::string dumped = node.dump();
    char* parsed = pdal_file_spec_parse_json(dumped.c_str());
    NL::json result = NL::json::parse(parsed ? parsed : "{}");
    pdal_string_free(parsed);

    if (!result.value("ok", false))
        return {-1, result.value("error", "")};

    setFilePath(result.value("path", ""));
    m_headers.clear();
    m_query.clear();
    if (result.contains("headers"))
        m_headers = result["headers"].get<StringMap>();
    if (result.contains("query"))
        m_query = result["query"].get<StringMap>();
    if (wasObject)
        node = NL::json::object();
    return true;
}

// Provide access to the private 'parse' function.
Utils::StatusWithReason FileSpecHelper::parse(FileSpec& spec, NL::json& node)
{
    return spec.m_p->parse(node);
}

std::ostream& operator<<(std::ostream& out, const FileSpec& spec)
{
    if (spec.onlyFilename())
        return out << spec.u8string();

    NL::json json;
    json["path"] = spec.u8string();
    if (!spec.m_p->m_headers.empty())
        json["headers"] = spec.m_p->m_headers;
    if (!spec.m_p->m_query.empty())
        json["query"] = spec.m_p->m_query;

    out << json;
    return out;
}

} // namespace pdal
