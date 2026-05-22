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

#include <pdal/util/Utils.hpp>

#include <nlohmann/json.hpp>

#include <cassert>
#include <cctype>
#include <cstdlib>
#include <iomanip>
#include <memory>
#include <random>
#include <sstream>

#ifndef PDAL_UTILS_NO_RUST_CAPI
#include <rust/pdal-capi/include/pdal_capi.h>
#endif

#ifndef _WIN32
#include <cxxabi.h>
#include <sys/ioctl.h>
#include <sys/wait.h> // WIFEXITED, WEXITSTATUS
#else
#include <windows.h> // GetConsoleScreenBufferInfo
#endif

#pragma warning(disable : 4127) // conditional expression is constant

#include <stdio.h>

#include "private/BacktraceImpl.hpp"

typedef std::vector<std::string> StringList;

namespace pdal
{

#ifndef PDAL_UTILS_NO_RUST_CAPI
namespace
{

std::string takeRustString(char* value)
{
    if (!value)
        return std::string();
    std::string result(value);
    pdal_string_free(value);
    return result;
}

StringList takeRustStringList(char* value)
{
    std::string json = takeRustString(value);
    try
    {
        return NL::json::parse(json).get<StringList>();
    }
    catch (NL::json::exception&)
    {
        return StringList();
    }
}

} // unnamed namespace
#endif

bool Utils::compare_approx(double v1, double v2, double tolerance)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return pdal_utils_compare_approx(v1, v2, tolerance);
#else
    double diff = std::abs(v1 - v2);
    return diff <= std::abs(tolerance);
#endif
}

std::string Utils::toString(double from, size_t precision)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return takeRustString(pdal_utils_to_string_f64(from,
        static_cast<uint32_t>(precision)));
#else
    OStringStreamClassicLocale oss;
    if (std::isnan(from))
        return "NaN";
    if (std::isinf(from))
        return (from < 0 ? "-Infinity" : "Infinity");
    oss << std::setprecision(precision) << from;
    return oss.str();
#endif
}

std::string Utils::toString(int from)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return takeRustString(pdal_utils_to_string_i32(from));
#else
    return std::to_string(from);
#endif
}

void Utils::random_seed(unsigned int seed)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    pdal_utils_random_seed(seed);
#else
    srand(seed);
#endif
}

double Utils::random(double minimum, double maximum)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return pdal_utils_random(minimum, maximum);
#else
    double r = (double)rand(); // [0..32767]
    double v = (maximum - minimum) / (double)RAND_MAX;
    double s = r * v;       // [0..(max-min)]
    double t = minimum + s; // [min..max]

    assert(t >= minimum);
    assert(t <= maximum);

    return t;
#endif
}

std::string Utils::tolower(const std::string& s)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return takeRustString(pdal_utils_to_lower(s.c_str()));
#else
    std::string out;
    for (size_t i = 0; i < s.size(); ++i)
        out += (char)std::tolower(s[i]);
    return out;
#endif
}

std::string Utils::toupper(const std::string& s)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return takeRustString(pdal_utils_to_upper(s.c_str()));
#else
    std::string out;
    for (size_t i = 0; i < s.size(); ++i)
        out += (char)std::toupper(s[i]);
    return out;
#endif
}

bool Utils::iequals(const std::string& s, const std::string& s2)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return pdal_utils_iequals(s.c_str(), s2.c_str());
#else
    if (s.length() != s2.length())
        return false;
    for (size_t i = 0; i < s.length(); ++i)
        if (std::toupper(s[i]) != std::toupper(s2[i]))
            return false;
    return true;
#endif
}

bool Utils::startsWith(const std::string& s, const std::string& prefix)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return pdal_utils_starts_with(s.c_str(), prefix.c_str());
#else
    if (prefix.empty())
        return true;
    if (prefix.size() > s.size())
        return false;
    return (strncmp(prefix.data(), s.data(), prefix.size()) == 0);
#endif
}

int Utils::getenv(const std::string& name, std::string& val)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    char* value = pdal_utils_getenv(name.c_str());
    if (value)
    {
        val = value;
        pdal_string_free(value);
        return 0;
    }
    else
    {
        val.clear();
        return -1;
    }
#else
    char* value = ::getenv(name.c_str());
    if (value)
        val = value;
    else
        val.clear();
    return value ? 0 : -1;
#endif
}

int Utils::setenv(const std::string& env, const std::string& val)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return pdal_utils_setenv(env.c_str(), val.c_str());
#else
#ifdef _WIN32
    return ::_putenv_s(env.c_str(), val.c_str()) ? -1 : 0;
#else
    return ::setenv(env.c_str(), val.c_str(), 1);
#endif
#endif
}

int Utils::unsetenv(const std::string& env)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return pdal_utils_unsetenv(env.c_str());
#else
#ifdef _WIN32
    return ::_putenv_s(env.c_str(), "") ? -1 : 0;
#else
    return ::unsetenv(env.c_str());
#endif
#endif
}

void Utils::eatwhitespace(std::istream& s)
{
    while (true)
    {
        const char c = (char)s.peek();
        if (!isspace(c))
            break;

        // throw it away
        s.get();
    }
}

void Utils::trimLeading(std::string& s)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    s = takeRustString(pdal_utils_trim_leading(s.c_str()));
#else
    size_t pos = 0;
    while (std::isspace(s[pos]))
        pos++;
    s.erase(s.begin(), s.begin() + pos);
#endif
}

void Utils::trimTrailing(std::string& s)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    s = takeRustString(pdal_utils_trim_trailing(s.c_str()));
#else
    if (s.empty())
        return;

    size_t pos = s.size() - 1;
    while (std::isspace(s[pos]))
    {
        if (pos == 0)
        {
            s.clear();
            return;
        }
        pos--;
    }
    s.erase(s.begin() + pos + 1, s.end());
#endif
}

bool Utils::eatcharacter(std::istream& s, char x)
{
    const char c = (char)s.peek();
    if (c != x)
        return false;

    // throw it away
    s.get();

    return true;
}

std::string Utils::base64_encode(const unsigned char* bytes_to_encode,
                                 size_t in_len)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return takeRustString(pdal_utils_base64_encode(bytes_to_encode, in_len));
#else
    const std::string chars =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    std::string ret;
    for (size_t offset = 0; offset < in_len; offset += 3)
    {
        uint8_t b0 = bytes_to_encode[offset];
        uint8_t b1 = offset + 1 < in_len ? bytes_to_encode[offset + 1] : 0;
        uint8_t b2 = offset + 2 < in_len ? bytes_to_encode[offset + 2] : 0;

        ret += chars[b0 >> 2];
        ret += chars[((b0 & 0x03) << 4) | (b1 >> 4)];
        ret +=
            offset + 1 < in_len ? chars[((b1 & 0x0f) << 2) | (b2 >> 6)] : '=';
        ret += offset + 2 < in_len ? chars[b2 & 0x3f] : '=';
    }
    return ret;
#endif
}

std::vector<uint8_t> Utils::base64_decode(std::string const& encoded_string)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    uint64_t len = 0;
    uint8_t* decoded = pdal_utils_base64_decode(encoded_string.c_str(), &len);
    std::vector<uint8_t> ret;
    if (decoded)
    {
        ret.assign(decoded, decoded + len);
        pdal_u8_array_free(decoded, len);
    }
    return ret;
#else
    auto value = [](unsigned char c) -> int
    {
        if (c >= 'A' && c <= 'Z')
            return c - 'A';
        if (c >= 'a' && c <= 'z')
            return c - 'a' + 26;
        if (c >= '0' && c <= '9')
            return c - '0' + 52;
        if (c == '+')
            return 62;
        if (c == '/')
            return 63;
        return -1;
    };

    std::vector<uint8_t> ret;
    uint8_t quartet[4]{};
    size_t count = 0;
    for (unsigned char c : encoded_string)
    {
        if (c == '=')
            break;
        int decoded = value(c);
        if (decoded < 0)
            break;
        quartet[count++] = static_cast<uint8_t>(decoded);
        if (count == 4)
        {
            ret.push_back((quartet[0] << 2) | ((quartet[1] & 0x30) >> 4));
            ret.push_back(((quartet[1] & 0x0f) << 4) |
                          ((quartet[2] & 0x3c) >> 2));
            ret.push_back(((quartet[2] & 0x03) << 6) | quartet[3]);
            count = 0;
        }
    }
    if (count > 1)
    {
        ret.push_back((quartet[0] << 2) | ((quartet[1] & 0x30) >> 4));
        if (count > 2)
            ret.push_back(((quartet[1] & 0x0f) << 4) |
                          ((quartet[2] & 0x3c) >> 2));
        if (count > 3)
            ret.push_back(((quartet[2] & 0x03) << 6) | quartet[3]);
    }
    return ret;
#endif
}

FILE* Utils::portable_popen(const std::string& command, const std::string& mode)
{
#ifdef _WIN32
    return _popen(command.c_str(), mode.c_str());
#else
    return popen(command.c_str(), mode.c_str());
#endif
}

int Utils::portable_pclose(FILE* fp)
{
    int status = 0;

#ifdef _WIN32
    status = _pclose(fp);
#else
    status = pclose(fp);
    if (status == -1)
    {
        throw std::runtime_error("Error closing pipe for subprocess");
    }
    if (WIFEXITED(status) != 0)
    {
        status = WEXITSTATUS(status);
    }
    else
    {
        status = 0;
    }
#endif

    return status;
}

int Utils::run_shell_command(const std::string& cmd, std::string& output)
{
    const int maxbuf = 4096;
    char buf[maxbuf];

    output = "";

    FILE* fp = portable_popen(cmd.c_str(), "r");

    if (fp == nullptr)
        return 1;

    while (!feof(fp))
    {
        if (fgets(buf, maxbuf, fp) == nullptr)
        {
            if (feof(fp))
                break;
            if (ferror(fp))
                break;
        }
        output += buf;
    }
    return portable_pclose(fp);
}

std::string Utils::replaceAll(std::string result,
                              const std::string& replaceWhat,
                              const std::string& replaceWithWhat)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return takeRustString(pdal_utils_replace_all(
        result.c_str(), replaceWhat.c_str(), replaceWithWhat.c_str()));
#else
    if (replaceWhat.empty())
        return result;

    size_t pos = 0;
    while (true)
    {
        pos = result.find(replaceWhat, pos);
        if (pos == std::string::npos)
            break;
        result.replace(pos, replaceWhat.size(), replaceWithWhat);
        pos += replaceWithWhat.size();
        if (pos >= result.size())
            break;
    }
    return result;
#endif
}

StringList Utils::split(const std::string& s, char tChar)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return takeRustStringList(pdal_utils_split_char(s.c_str(), tChar));
#else
    auto pred = [tChar](char c) { return (c == tChar); };
    return split(s, pred);
#endif
}

StringList Utils::split2(const std::string& s, char tChar)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return takeRustStringList(pdal_utils_split2_char(s.c_str(), tChar));
#else
    auto pred = [tChar](char c) { return (c == tChar); };
    return split2(s, pred);
#endif
}

std::string Utils::escapeJSON(const std::string& str)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return takeRustString(pdal_utils_escape_json(str.c_str()));
#else
    std::string s;
    for (char c : str)
    {
        switch (c)
        {
        case '\t':
            s += "\\t";
            break;
        case '\n':
            s += "\\n";
            break;
        case '\f':
            s += "\\f";
            break;
        case '\r':
            s += "\\r";
            break;
        case '"':
            s += "\\\"";
            break;
        case '\\':
            s += "\\\\";
            break;
        default:
            if (static_cast<unsigned char>(c) < 32)
            {
                std::stringstream oss;
                oss << "\\u" << std::uppercase << std::hex << std::setfill('0')
                    << std::setw(4) << static_cast<int>(c);
                s += oss.str();
            }
            else
                s += c;
            break;
        }
    }
    return s;
#endif
}

StringList Utils::wordWrap(std::string const& s, size_t lineLength,
                           size_t firstLength)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return takeRustStringList(
        pdal_utils_word_wrap(s.c_str(), lineLength, firstLength));
#else
    std::vector<std::string> output;
    if (s.empty())
        return output;

    if (firstLength == 0)
        firstLength = lineLength;

    size_t len = firstLength;

    StringStreamClassicLocale iss(s);
    std::string line;
    do
    {
        std::string word;
        iss >> word;

        if ((line.length() + word.length() > len) && line.length())
        {
            trimTrailing(line);
            output.push_back(line);
            len = lineLength;
            line.clear();
        }
        while (word.length() > len)
        {
            output.push_back(word.substr(0, len));
            word = word.substr(len);
            len = lineLength;
        }
        line += word + " ";
    } while (iss);
    trimTrailing(line);
    if (!line.empty())
        output.push_back(line);
    return output;
#endif
}

StringList Utils::wordWrap2(std::string const& s, size_t lineLength,
                            size_t firstLength)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return takeRustStringList(
        pdal_utils_word_wrap2(s.c_str(), lineLength, firstLength));
#else
    std::vector<std::string> output;
    if (s.empty())
        return output;

    if (firstLength == 0)
        firstLength = lineLength;

    auto pushWord = [&s, &output](size_t start, size_t end)
    {
        if (start != end)
            output.push_back(s.substr(start, end - start + 1));
    };

    size_t len = firstLength;
    size_t startPos = 0;
    while (true)
    {
        size_t endPos = (std::min)(startPos + len - 1, s.size() - 1);
        if (endPos + 1 == s.size())
        {
            pushWord(startPos, endPos);
            return output;
        }
        size_t pos = endPos;
        while (pos > startPos)
        {
            if (std::isspace(s[pos]) && !std::isspace(s[pos + 1]))
            {
                endPos = pos;
                break;
            }
            pos--;
        }
        pushWord(startPos, endPos);
        len = lineLength;
        startPos = endPos + 1;
    }
    return output;
#endif
}

/// Demangle strings using the compiler-provided demangle function.
/// \param[in] s  String to be demangled.
/// \return  Demangled string
std::string Utils::demangle(const std::string& s)
{
#ifndef _WIN32
    int status;
    std::unique_ptr<char[], void (*)(void*)> result(
        abi::__cxa_demangle(s.c_str(), nullptr, nullptr, &status), std::free);
    if (status == 0)
        return std::string(result.get());
#endif

    return s;
}

int Utils::screenWidth()
{
#ifdef _WIN32
    return 80;
#else
    struct winsize ws;
    int err(0);
    err = ioctl(0, TIOCGWINSZ, &ws);
    if (err == 0)
        return ws.ws_col;
    else
    {
        if (errno == EBADF)
            throw std::runtime_error(
                "screen width not a valid file descriptor");
        else if (errno == EFAULT)
            throw std::runtime_error("Inaccessible memory access in ioctl");
        else if (errno == EINVAL)
            throw std::runtime_error(
                "Request invalid in gathering screenWidth");
        else
            // we are not a tty, so just return 80 *shrug*
            return 80;
    }

#endif
}

std::string Utils::escapeNonprinting(const std::string& s)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return takeRustString(pdal_utils_escape_nonprinting(s.c_str()));
#else
    std::string out;
    for (char c : s)
    {
        if (c == '\n')
            out += "\\n";
        else if (c == '\a')
            out += "\\a";
        else if (c == '\b')
            out += "\\b";
        else if (c == '\r')
            out += "\\r";
        else if (c == '\v')
            out += "\\v";
        else if (c < 32)
        {
            std::stringstream oss;
            oss << std::hex << std::setfill('0') << std::setw(2)
                << static_cast<int>(c);
            out += "\\x" + oss.str();
        }
        else
            out += c;
    }
    return out;
#endif
}

double Utils::normalizeLongitude(double longitude)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return pdal_utils_normalize_longitude(longitude);
#else
    longitude = fmod(longitude, 360.0);
    if (longitude <= -180)
        longitude += 360;
    else if (longitude > 180)
        longitude -= 360;
    return longitude;
#endif
}

std::vector<std::string> Utils::simpleWordexp(const std::string& cmdline)
{
#ifndef PDAL_UTILS_NO_RUST_CAPI
    return takeRustStringList(pdal_utils_simple_wordexp(cmdline.c_str()));
#else
    std::string temp;
    bool instring = false;
    bool escape = false;
    std::vector<std::string> cmdArgs;
    for (size_t i = 0; i < cmdline.size(); ++i)
    {
        if (instring)
        {
            if (escape)
            {
                if (cmdline[i] != '"' && cmdline[i] != '\\')
                    temp += '\\';
                escape = false;
                temp += cmdline[i];
            }
            else if (cmdline[i] == '"')
                instring = false;
            else if (cmdline[i] == '\\')
                escape = true;
            else
                temp += cmdline[i];
        }
        else
        {
            if (escape)
            {
                escape = false;
                temp += cmdline[i];
            }
            else if (cmdline[i] == '"')
                instring = true;
            else if (cmdline[i] == '\\')
                escape = true;
            else if (std::isspace(cmdline[i]))
            {
                if (temp.size())
                {
                    cmdArgs.push_back(temp);
                    temp.clear();
                }
            }
            else
                temp += cmdline[i];
        }
    }
    if (!instring && temp.size())
        cmdArgs.push_back(temp);
    return cmdArgs;
#endif
}

} // namespace pdal
