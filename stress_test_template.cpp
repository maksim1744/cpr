namespace main_sol {
stringstream cin, cout, cerr;
//->main
}

namespace easy_sol {
stringstream cin, cout, cerr;
//->easy
}

namespace gen_sol {
using namespace gen;
stringstream cin, cout, cerr;
//->gen
}

bool is_double(const string& s) {
    char* end = nullptr;
    double val = strtod(s.c_str(), &end);
    return end != s.c_str() && *end == '\0' && val != HUGE_VAL;
}

//->settings

int main() {
    ios_base::sync_with_stdio(false); cin.tie(NULL); cout.tie(NULL);

    for (int test = 1;; ++test) {
        int seed = start_seed + test - 1;
        cerr << '\r' << string(35, ' ');
        cerr << "\rCase #" << test << " [seed=" << seed << "]: ";
        gen_sol::cin = stringstream{};
        gen_sol::cout = stringstream{};
        gen_sol::cerr = stringstream{};
        main_sol::cin = stringstream{};
        main_sol::cout = stringstream{};
        main_sol::cerr = stringstream{};
        easy_sol::cin = stringstream{};
        easy_sol::cout = stringstream{};
        easy_sol::cerr = stringstream{};

        char *argv[2];
        string s = to_string(seed);
        argv[1] = (char*)&s[0];
        gen_sol::main(2, argv);
        cout << "."; cout.flush();
        easy_sol::cin << gen_sol::cout.str();
        easy_sol::main();
        cout << "."; cout.flush();
        main_sol::cin << gen_sol::cout.str();
        main_sol::main();
        cout << "."; cout.flush();

        auto into_tokens = [&](string sss) {
            stringstream ss;
            ss << sss;
            vector<string> res;
            string s;
            while (ss >> s) {
                res.push_back(s);
            }
            return res;
        };

        auto compare_eps = [&](const vector<string> &a, const vector<string> &b) {
            if (a.size() != b.size()) return false;
            for (int i = 0; i < a.size(); ++i) {
                if (!is_double(a[i])) {
                    if (a[i] != b[i]) return false;
                    continue;
                }
                if (!is_double(b[i]))
                    return false;
                long double x = stod(a[i]);
                long double y = stod(b[i]);
                if (abs(x - y) < eps) continue;
                if (abs(x - y) / max(abs(x), abs(y)) < eps) continue;
                return false;
            }
            return true;
        };

        auto compare = [&](const vector<string> &a, const vector<string> &b) {
            if (use_eps)
                return compare_eps(a, b);
            return a == b;
        };

        if (!compare(into_tokens(main_sol::cout.str()), into_tokens(easy_sol::cout.str()))) {
            cout << " failed";
            cout << endl;
            if (!quiet) {
                cout << "==========  in ==========" << endl;
                cout << gen_sol::cout.str();
                cout << "========== ans ==========" << endl;
                cout << easy_sol::cout.str();
                cout << "========== out ==========" << endl;
                cout << main_sol::cout.str();
            }

            {
                ofstream f("in");
                f << gen_sol::cout.str();
            }
            {
                ofstream f("out");
                f << main_sol::cout.str();
            }
            {
                ofstream f("ans");
                f << easy_sol::cout.str();
            }
            break;
        }
    }

    return 0;
}
